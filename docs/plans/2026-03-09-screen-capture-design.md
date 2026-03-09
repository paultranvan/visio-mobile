# Desktop Screen Capture — Design

**Date**: 2026-03-09
**Status**: Approved

## Scope

Implement platform-specific screen capture for the Desktop app using the `xcap` crate (cross-platform: macOS, Linux, Windows). This completes the screen sharing publish flow — the track and LiveKit integration already exist.

## Architecture

New module `crates/visio-desktop/src/screen_capture.rs`:

- `list_sources() -> Vec<ScreenSource>` — lists monitors and windows via `xcap::Monitor::all()` and `xcap::Window::all()`
- `ScreenCapture::start(source_id, video_source) -> Self` — starts a tokio timer capturing at ~15 fps, converts RGBA→I420, feeds into `NativeVideoSource`
- `ScreenCapture::stop()` — stops the capture timer

### ScreenSource

```rust
struct ScreenSource {
    id: String,          // "monitor-0", "window-12345"
    name: String,        // "Built-in Display", "Firefox"
    source_type: String, // "monitor" or "window"
}
```

### Data Flow

```
xcap capture_image() → RgbaImage (screenshot)
  → RGBA→I420 conversion (row-by-row)
  → VideoFrame { buffer: I420Buffer }
  → NativeVideoSource::capture_frame()
  → LiveKit publishes to remote participants
```

Timer: tokio interval at 15 fps (67ms). No self-view for screen share.

### State Management

`ScreenCapture` stored in `VisioState` (like `camera_capture`), stopped on disconnect.

## Tauri Commands

| Command | Action |
|---------|--------|
| `list_screen_sources` (new) | Returns `Vec<ScreenSource>` to frontend |
| `start_screen_share(source_id)` (updated) | Publishes track + starts `ScreenCapture::start(source_id, source)` |
| `stop_screen_share` (updated) | Stops `ScreenCapture` + unpublishes track |

## Frontend

1. Button "Share screen" opens modal
2. Modal calls `list_screen_sources`, lists monitors and windows with icons
3. User selects → `start_screen_share(source_id)`
4. Button turns red "Stop sharing"

## Dependencies

Add `xcap` to `crates/visio-desktop/Cargo.toml`.

## Out of Scope

- Screen share self-view preview
- Audio capture with screen share
- Annotation on shared screen
