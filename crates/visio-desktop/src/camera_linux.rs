//! Linux camera capture using V4L2.
//!
//! Opens the default camera (/dev/video0), captures YUYV frames,
//! converts to I420, and feeds frames into a LiveKit NativeVideoSource.
//! Also emits self-view frames through the visio-video desktop callback.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use livekit::webrtc::prelude::*;
use livekit::webrtc::video_source::native::NativeVideoSource;

use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;
use v4l::{Device, FourCC};

const CAPTURE_WIDTH: u32 = 1280;
const CAPTURE_HEIGHT: u32 = 720;

pub struct LinuxCameraCapture {
    stop_flag: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

unsafe impl Send for LinuxCameraCapture {}

impl LinuxCameraCapture {
    pub fn start(source: NativeVideoSource) -> Result<Self, String> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let flag = stop_flag.clone();

        let thread = std::thread::Builder::new()
            .name("visio-camera".into())
            .spawn(move || {
                if let Err(e) = capture_loop(source, flag) {
                    tracing::error!("camera capture error: {e}");
                }
            })
            .map_err(|e| format!("failed to spawn camera thread: {e}"))?;

        Ok(LinuxCameraCapture {
            stop_flag,
            thread: Some(thread),
        })
    }

    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        tracing::info!("Linux camera capture stopped");
    }
}

impl Drop for LinuxCameraCapture {
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.stop();
        }
    }
}

fn capture_loop(source: NativeVideoSource, stop_flag: Arc<AtomicBool>) -> Result<(), String> {
    let dev = Device::new(0).map_err(|e| format!("failed to open /dev/video0: {e}"))?;

    // Try YUYV format first (widely supported), fall back to whatever is available
    let mut fmt = dev.format().map_err(|e| format!("get format: {e}"))?;
    fmt.width = CAPTURE_WIDTH;
    fmt.height = CAPTURE_HEIGHT;
    fmt.fourcc = FourCC::new(b"YUYV");
    let fmt = dev
        .set_format(&fmt)
        .map_err(|e| format!("set format: {e}"))?;

    let fourcc = fmt.fourcc;
    let width = fmt.width;
    let height = fmt.height;
    tracing::info!(
        "Linux camera: {}x{} fourcc={fourcc}",
        width,
        height,
    );

    let mut stream =
        v4l::io::mmap::Stream::with_buffers(&dev, Type::VideoCapture, 4)
            .map_err(|e| format!("create stream: {e}"))?;

    let mut frame_count: u64 = 0;

    loop {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        let (buf, _meta) = match stream.next() {
            Ok(item) => item,
            Err(e) => {
                tracing::warn!("camera frame error: {e}");
                continue;
            }
        };

        frame_count += 1;

        let mut i420 = I420Buffer::new(width, height);
        let strides = i420.strides();

        if fourcc == FourCC::new(b"YUYV") {
            yuyv_to_i420(buf, width, height, &mut i420, strides);
        } else {
            // Unsupported format — skip
            if frame_count == 1 {
                tracing::warn!("unsupported camera format: {fourcc}, only YUYV is supported");
            }
            continue;
        }

        // Apply background processing (blur/replacement) if enabled
        {
            let (y_data, u_data, v_data) = i420.data_mut();
            visio_ffi::blur::BlurProcessor::process_i420(
                y_data,
                u_data,
                v_data,
                width as usize,
                height as usize,
                strides.0 as usize,
                strides.1 as usize,
                strides.2 as usize,
                0,
            );
        }

        let frame = VideoFrame {
            rotation: VideoRotation::VideoRotation0,
            timestamp_us: 0,
            buffer: i420,
        };
        source.capture_frame(&frame);

        // Self-view: render every 2nd frame
        if frame_count % 2 == 0 {
            visio_video::render_local_i420(&frame.buffer, "local-camera");
        }
    }

    Ok(())
}

/// Convert YUYV (YUY2) packed format to I420 planar.
///
/// YUYV: [Y0 U0 Y1 V0] [Y2 U1 Y3 V1] ...
/// Each 4 bytes encode 2 pixels sharing one U and one V sample.
fn yuyv_to_i420(
    yuyv: &[u8],
    width: u32,
    height: u32,
    i420: &mut I420Buffer,
    strides: (u32, u32, u32),
) {
    let w = width as usize;
    let h = height as usize;
    let (y_dst, u_dst, v_dst) = i420.data_mut();
    let yuyv_stride = w * 2; // 2 bytes per pixel in YUYV

    for row in 0..h {
        let src_row = &yuyv[row * yuyv_stride..(row * yuyv_stride + yuyv_stride).min(yuyv.len())];
        let y_row_start = row * strides.0 as usize;

        for col in (0..w).step_by(2) {
            let src_idx = col * 2;
            if src_idx + 4 > src_row.len() {
                break;
            }

            let y0 = src_row[src_idx];
            let u = src_row[src_idx + 1];
            let y1 = src_row[src_idx + 2];
            let v = src_row[src_idx + 3];

            y_dst[y_row_start + col] = y0;
            if col + 1 < w {
                y_dst[y_row_start + col + 1] = y1;
            }

            // Chroma is subsampled: store U/V only for even rows
            if row % 2 == 0 {
                let chroma_col = col / 2;
                let chroma_row = row / 2;
                let u_idx = chroma_row * strides.1 as usize + chroma_col;
                let v_idx = chroma_row * strides.2 as usize + chroma_col;
                if u_idx < u_dst.len() && v_idx < v_dst.len() {
                    u_dst[u_idx] = u;
                    v_dst[v_idx] = v;
                }
            }
        }
    }
}
