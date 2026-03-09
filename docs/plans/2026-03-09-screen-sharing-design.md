# Screen Sharing — Design

**Date**: 2026-03-09
**Status**: Approved

## Scope

- **Receive and display** screen shares on all platforms (Android, iOS, Desktop)
- **Publish** screen shares on Desktop only (macOS, Linux, Windows)
- Mobile apps do NOT publish screen shares

## UX Behavior

### Focus and Thumbnails

When a participant starts sharing their screen:
1. The screen share automatically becomes the **focus view** (large, center)
2. A **thumbnail** appears in the thumbnail bar for the screen share, with an **screen icon** + participant name to distinguish it from their camera
3. The participant's camera remains as a separate thumbnail

The user can click any thumbnail (camera or screen share) to force it as the focus view. When the screen share stops, if it was the focused item, the view returns to grid mode or the first participant.

### Desktop Screen Share Controls

A "Share screen" button in the control bar (next to camera/mic). On click:
1. Calls `list_screen_sources` — returns available screens and windows
2. Displays a modal/dropdown listing sources (name + type)
3. User selects a source → calls `start_screen_share(source_id)`
4. Button becomes "Stop sharing" (red)
5. Stop → calls `stop_screen_share`, button returns to normal

## Architecture

### Approach: Screen Share as Virtual Participant (Approach A)

The screen share is modeled as a separate entry in the participant list on the UI side. `ParticipantInfo` in Rust core gains a `screen_share_track_sid` field alongside the existing `video_track_sid`.

### Layer-by-Layer Changes

#### visio-core (Rust)

`ParticipantInfo` gains two fields:
```rust
pub screen_share_track_sid: Option<String>,
pub has_screen_share: bool,
```

Event handling changes:
- `TrackSubscribed` with `source=ScreenShare`: sets `screen_share_track_sid` and `has_screen_share = true`
- `TrackUnsubscribed` with `source=ScreenShare`: clears both fields
- `TrackMuted`/`TrackUnmuted`: handles `ScreenShare` source (currently only handles `Camera`)

New methods on `MeetingControls` (Desktop only):
- `list_screen_sources() -> Vec<ScreenSource>` — lists available screens and windows
- `publish_screen_share(source_id: String)` — publishes a screen share track
- `stop_screen_share()` — unpublishes the screen share track

#### visio-video

No changes. The pipeline is already track-source agnostic. Each platform creates an additional surface for the screen share track and attaches it via the existing `start_track_renderer` mechanism.

#### visio-ffi (UniFFI)

Expose the new `ParticipantInfo` fields (`screen_share_track_sid`, `has_screen_share`) in the UDL file.

#### Android

- Thumbnail bar: add a screen share thumbnail entry per active screen share, with screen icon + participant name
- Auto-focus on screen share arrival
- Click any thumbnail to force focus
- Return to grid when screen share ends (if it was focused)
- No publish capability

#### iOS

Same behavior as Android:
- Screen share thumbnail with icon + name
- Auto-focus, click-to-focus, auto-return
- No publish capability

#### Desktop Backend (Tauri)

Three new Tauri commands:
- `list_screen_sources` → calls `MeetingControls::list_screen_sources()`
- `start_screen_share(source_id)` → calls `MeetingControls::publish_screen_share(source_id)`
- `stop_screen_share` → calls `MeetingControls::stop_screen_share()`

Event listener: start/stop video renderer for screen share tracks on TrackSubscribed/TrackUnsubscribed.

#### Desktop Frontend (React)

- Share button in control bar + source picker modal
- Same focus/thumbnail logic as mobile
- Toggle button state (share / stop sharing)

### UI State Model (all platforms)

```
focusedItem: Option<(participant_id, TrackSource)>
```

- `None` → grid mode
- `Some((id, Camera))` → participant camera in focus
- `Some((id, ScreenShare))` → screen share in focus

## Out of Scope

- Screen sharing from mobile (Android/iOS)
- Annotation on shared screen
- Screen share recording
- Audio sharing with screen share
