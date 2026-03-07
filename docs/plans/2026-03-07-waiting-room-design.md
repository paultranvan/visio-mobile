# Waiting Room for Trusted Rooms — Design

## Scope

Participants **non-authenticated** joining **trusted** rooms. Authenticated users bypass the lobby automatically (Meet backend `can_bypass_lobby` behavior).

## Meet API Endpoints

| Endpoint | Method | Role |
|----------|--------|------|
| `POST /api/v1.0/rooms/{id}/request-entry/` | POST | Participant requests entry (body: `{"username": "..."}`) |
| `GET /api/v1.0/rooms/{id}/waiting-participants/` | GET | Host lists waiting participants (requires auth) |
| `POST /api/v1.0/rooms/{id}/enter/` | POST | Host accepts/rejects (body: `{"participant_id": "...", "allow_entry": true/false}`) |

## Lobby Flow

### Participant (anonymous)

1. Calls `POST /api/v1.0/rooms/{room_id}/request-entry/` with `{"username": "..."}`
2. Receives `{"status": "waiting", "id": "...", "livekit": null}`
3. Displays a **waiting screen** ("Waiting for host approval...")
4. **Polls** `request-entry` every 3 seconds (also refreshes timeout server-side)
5. When `status == "accepted"` and `livekit != null` -> connects to LiveKit normally
6. When `status == "denied"` -> shows message, returns to home screen

### Host (in call)

1. Receives notification via **LiveKit data channel** (topic: lobby notification type)
2. Shows a **badge/notification** on participants icon
3. UI to see waiting participants (`GET /api/v1.0/rooms/{room_id}/waiting-participants/`)
4. **Accept / Reject** buttons per participant (`POST /api/v1.0/rooms/{room_id}/enter/`)

## Detection

When `request_token()` returns `livekit: None` AND user is NOT authenticated -> switch to lobby mode (poll `request-entry`). If user IS authenticated, the `request-entry` call returns `status: "accepted"` + `livekit` immediately (bypass).

## Components to Modify

| Layer | Files | Change |
|-------|-------|--------|
| **visio-core** | `lobby.rs` (new) | `LobbyService` with `request_entry()`, `poll_entry()`, `list_waiting()`, `handle_entry()` |
| **visio-core** | `room.rs` | Listen for lobby data channel notifications, emit event to UI |
| **visio-core** | `errors.rs` | Add `WaitingForApproval` error variant |
| **visio-ffi** | `lib.rs`, `visio.udl` | Expose lobby functions + lobby notification callback |
| **Desktop** | `lib.rs`, `App.tsx`, `App.css` | Waiting screen (participant) + accept/reject UI (host) |
| **Android** | `CallScreen.kt`, `HomeScreen.kt` | Same |
| **iOS** | `CallView.swift`, `HomeView.swift` | Same |
| **i18n** | `*.json` | New keys for waiting room strings |

## UI — Participant Waiting Screen

- Spinner/animation
- "Waiting for host approval..." (i18n, 6 languages)
- Cancel button to leave
- Same design across all 3 platforms

## UI — Host Accept/Reject

- Badge on participants icon when lobby is non-empty
- In participants list: "Waiting" section with name + Accept/Reject buttons
- Poll `waiting-participants` every 5 seconds + immediate refresh on data channel notification

## Server-side Notes (from Meet source)

- Lobby state stored in Django cache with timeout (`LOBBY_WAITING_TIMEOUT`)
- Participant identified by cookie (`LOBBY_COOKIE_NAME`) — our app must store and resend this cookie
- `can_bypass_lobby(room, user)` returns true for public rooms OR (trusted + authenticated)
- Notifications sent to room via `notify_participants()` using LiveKit data channels
