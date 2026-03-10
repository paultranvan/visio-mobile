use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

/// Thread-safe audio mixer for decoded remote audio PCM samples.
///
/// Each remote audio track pushes into its own per-track ring buffer.
/// Platform audio output (Android AudioTrack, desktop cpal) pulls mixed
/// audio from all tracks combined.
///
/// This avoids the N-participant problem where N streams concatenated
/// into one buffer causes overflow and choppy audio.
pub struct AudioPlayoutBuffer {
    tracks: Mutex<HashMap<String, VecDeque<i16>>>,
    /// Maximum number of i16 samples per track (1 second at 48kHz mono).
    max_samples_per_track: usize,
}

impl Default for AudioPlayoutBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlayoutBuffer {
    pub fn new() -> Self {
        Self {
            tracks: Mutex::new(HashMap::new()),
            max_samples_per_track: 48_000, // 1 second per track
        }
    }

    /// Push PCM samples for a specific track into its buffer.
    ///
    /// If the track's buffer would exceed max capacity, oldest samples are dropped.
    pub fn push_samples_for_track(&self, track_id: &str, samples: &[i16]) {
        let mut tracks = self.tracks.lock().unwrap();
        let buf = tracks
            .entry(track_id.to_string())
            .or_insert_with(|| VecDeque::with_capacity(self.max_samples_per_track));

        buf.extend(samples.iter().copied());

        let overflow = buf.len().saturating_sub(self.max_samples_per_track);
        if overflow > 0 {
            buf.drain(..overflow);
        }
    }

    /// Push PCM samples (legacy single-buffer API, used by platforms
    /// that don't pass a track ID). Pushes to a default "_mixed" track.
    pub fn push_samples(&self, samples: &[i16]) {
        self.push_samples_for_track("_mixed", samples);
    }

    /// Pull up to `out.len()` mixed samples from all track buffers.
    ///
    /// Mixes by summing all tracks with clamping. Returns the number of
    /// samples actually written. Unfilled positions are zeroed (silence).
    pub fn pull_samples(&self, out: &mut [i16]) -> usize {
        let mut tracks = self.tracks.lock().unwrap();
        let requested = out.len();

        // Zero the output
        for s in out.iter_mut() {
            *s = 0;
        }

        let mut max_available = 0usize;

        for buf in tracks.values_mut() {
            let available = buf.len().min(requested);
            if available > max_available {
                max_available = available;
            }

            // Mix: add each track's samples to the output with saturation
            for (i, sample) in buf.drain(..available).enumerate() {
                let mixed = out[i] as i32 + sample as i32;
                out[i] = mixed.clamp(-32768, 32767) as i16;
            }
        }

        // Remove empty track buffers to avoid leaking memory
        tracks.retain(|_, buf| !buf.is_empty());

        max_available
    }

    /// Remove a specific track's buffer (e.g., on unsubscribe).
    pub fn remove_track(&self, track_id: &str) {
        self.tracks.lock().unwrap().remove(track_id);
    }

    /// Clear all buffered samples (e.g., on disconnect).
    pub fn clear(&self) {
        self.tracks.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_pull_single_track() {
        let buf = AudioPlayoutBuffer::new();
        buf.push_samples_for_track("t1", &[100, 200, 300, 400, 500]);

        let mut out = vec![0i16; 3];
        let n = buf.pull_samples(&mut out);
        assert_eq!(n, 3);
        assert_eq!(out, vec![100, 200, 300]);

        let mut out2 = vec![0i16; 5];
        let n2 = buf.pull_samples(&mut out2);
        assert_eq!(n2, 2);
        assert_eq!(out2, vec![400, 500, 0, 0, 0]);
    }

    #[test]
    fn mix_two_tracks() {
        let buf = AudioPlayoutBuffer::new();
        buf.push_samples_for_track("t1", &[100, 200, 300]);
        buf.push_samples_for_track("t2", &[50, -50, 100]);

        let mut out = vec![0i16; 3];
        let n = buf.pull_samples(&mut out);
        assert_eq!(n, 3);
        assert_eq!(out, vec![150, 150, 400]);
    }

    #[test]
    fn mix_clamps_overflow() {
        let buf = AudioPlayoutBuffer::new();
        buf.push_samples_for_track("t1", &[30000]);
        buf.push_samples_for_track("t2", &[30000]);

        let mut out = vec![0i16; 1];
        buf.pull_samples(&mut out);
        assert_eq!(out[0], 32767); // clamped
    }

    #[test]
    fn mix_tracks_different_lengths() {
        let buf = AudioPlayoutBuffer::new();
        buf.push_samples_for_track("t1", &[100, 200, 300]);
        buf.push_samples_for_track("t2", &[50]);

        let mut out = vec![0i16; 3];
        let n = buf.pull_samples(&mut out);
        assert_eq!(n, 3);
        assert_eq!(out, vec![150, 200, 300]);
    }

    #[test]
    fn overflow_drops_oldest_per_track() {
        let buf = AudioPlayoutBuffer {
            tracks: Mutex::new(HashMap::new()),
            max_samples_per_track: 4,
        };

        buf.push_samples_for_track("t1", &[1, 2, 3, 4]);
        buf.push_samples_for_track("t1", &[5, 6]);

        let mut out = vec![0i16; 6];
        let n = buf.pull_samples(&mut out);
        assert_eq!(n, 4);
        assert_eq!(out, vec![3, 4, 5, 6, 0, 0]);
    }

    #[test]
    fn pull_empty_returns_silence() {
        let buf = AudioPlayoutBuffer::new();
        let mut out = vec![99i16; 3];
        let n = buf.pull_samples(&mut out);
        assert_eq!(n, 0);
        assert_eq!(out, vec![0, 0, 0]);
    }

    #[test]
    fn clear_empties_all_tracks() {
        let buf = AudioPlayoutBuffer::new();
        buf.push_samples_for_track("t1", &[1, 2, 3]);
        buf.push_samples_for_track("t2", &[4, 5, 6]);
        buf.clear();

        let mut out = vec![0i16; 3];
        let n = buf.pull_samples(&mut out);
        assert_eq!(n, 0);
    }

    #[test]
    fn remove_track() {
        let buf = AudioPlayoutBuffer::new();
        buf.push_samples_for_track("t1", &[100, 200]);
        buf.push_samples_for_track("t2", &[50, 60]);
        buf.remove_track("t1");

        let mut out = vec![0i16; 2];
        buf.pull_samples(&mut out);
        assert_eq!(out, vec![50, 60]);
    }

    // Legacy API compatibility
    #[test]
    fn push_samples_legacy() {
        let buf = AudioPlayoutBuffer::new();
        buf.push_samples(&[100, 200, 300]);

        let mut out = vec![0i16; 3];
        let n = buf.pull_samples(&mut out);
        assert_eq!(n, 3);
        assert_eq!(out, vec![100, 200, 300]);
    }
}
