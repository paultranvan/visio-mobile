# Waiting Room Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement host approval flow for anonymous participants joining trusted rooms, with waiting screen + polling for participants and accept/reject UI for hosts, across Desktop, Android, and iOS.

**Architecture:** New `LobbyService` in visio-core handles Meet API lobby endpoints (`request-entry`, `waiting-participants`, `enter`). A new `ConnectionState::WaitingForHost` variant signals the UI to show a waiting screen. The host receives lobby notifications via LiveKit data channels and manages waiting participants through the existing event system. Each platform (Desktop/Android/iOS) adds a waiting screen for participants and an accept/reject UI in the participants panel for hosts.

**Tech Stack:** Rust (visio-core, reqwest, serde, tokio), UniFFI, Tauri 2.x, Kotlin Compose, SwiftUI

**Design doc:** `docs/plans/2026-03-07-waiting-room-design.md`

---

### Task 1: Add i18n keys for waiting room (all 6 languages)

**Files:**
- Modify: `i18n/en.json`
- Modify: `i18n/fr.json`
- Modify: `i18n/de.json`
- Modify: `i18n/es.json`
- Modify: `i18n/it.json`
- Modify: `i18n/nl.json`
- Copy to: `android/app/src/main/assets/i18n/*.json` (same content)

**Step 1: Add keys to all 6 language files**

Add these keys to `i18n/en.json` (before the closing `}`):

```json
"lobby.waiting": "Waiting for host approval...",
"lobby.waitingDesc": "The host will let you in soon",
"lobby.denied": "Entry denied by host",
"lobby.cancel": "Cancel",
"lobby.waitingParticipants": "Waiting room",
"lobby.admit": "Admit",
"lobby.deny": "Deny",
"lobby.admitAll": "Admit all",
"lobby.badge": "{count} waiting"
```

Add equivalent translations in fr, de, es, it, nl:

**French (fr.json):**
```json
"lobby.waiting": "En attente d'approbation de l'hôte...",
"lobby.waitingDesc": "L'hôte vous laissera entrer bientôt",
"lobby.denied": "Entrée refusée par l'hôte",
"lobby.cancel": "Annuler",
"lobby.waitingParticipants": "Salle d'attente",
"lobby.admit": "Admettre",
"lobby.deny": "Refuser",
"lobby.admitAll": "Admettre tous",
"lobby.badge": "{count} en attente"
```

**German (de.json):**
```json
"lobby.waiting": "Warte auf Genehmigung des Gastgebers...",
"lobby.waitingDesc": "Der Gastgeber wird Sie bald einlassen",
"lobby.denied": "Zutritt vom Gastgeber verweigert",
"lobby.cancel": "Abbrechen",
"lobby.waitingParticipants": "Wartezimmer",
"lobby.admit": "Zulassen",
"lobby.deny": "Ablehnen",
"lobby.admitAll": "Alle zulassen",
"lobby.badge": "{count} wartend"
```

**Spanish (es.json):**
```json
"lobby.waiting": "Esperando la aprobación del anfitrión...",
"lobby.waitingDesc": "El anfitrión te dejará entrar pronto",
"lobby.denied": "Entrada denegada por el anfitrión",
"lobby.cancel": "Cancelar",
"lobby.waitingParticipants": "Sala de espera",
"lobby.admit": "Admitir",
"lobby.deny": "Denegar",
"lobby.admitAll": "Admitir a todos",
"lobby.badge": "{count} esperando"
```

**Italian (it.json):**
```json
"lobby.waiting": "In attesa dell'approvazione dell'host...",
"lobby.waitingDesc": "L'host ti farà entrare presto",
"lobby.denied": "Ingresso negato dall'host",
"lobby.cancel": "Annulla",
"lobby.waitingParticipants": "Sala d'attesa",
"lobby.admit": "Ammetti",
"lobby.deny": "Rifiuta",
"lobby.admitAll": "Ammetti tutti",
"lobby.badge": "{count} in attesa"
```

**Dutch (nl.json):**
```json
"lobby.waiting": "Wachten op goedkeuring van de host...",
"lobby.waitingDesc": "De host laat je snel binnen",
"lobby.denied": "Toegang geweigerd door de host",
"lobby.cancel": "Annuleren",
"lobby.waitingParticipants": "Wachtkamer",
"lobby.admit": "Toelaten",
"lobby.deny": "Weigeren",
"lobby.admitAll": "Alles toelaten",
"lobby.badge": "{count} wachtend"
```

**Step 2: Copy to Android assets**

```bash
cp i18n/*.json android/app/src/main/assets/i18n/
```

**Step 3: Commit**

```bash
git add -f i18n/*.json android/app/src/main/assets/i18n/*.json
git commit -m "feat(i18n): add waiting room strings for all 6 languages"
```

---

### Task 2: Add `WaitingForHost` to ConnectionState and LobbyEvent

This task adds the new connection state variant and lobby-specific events to visio-core.

**Files:**
- Modify: `crates/visio-core/src/events.rs:35-41` (ConnectionState enum)
- Modify: `crates/visio-core/src/events.rs:4-33` (VisioEvent enum)

**Step 1: Add `WaitingForHost` variant to ConnectionState**

In `crates/visio-core/src/events.rs`, add a new variant to `ConnectionState` (after line 40):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32 },
    WaitingForHost,
}
```

**Step 2: Add lobby events to VisioEvent**

Add these variants to the `VisioEvent` enum (before `ConnectionLost`):

```rust
    /// A participant is waiting in the lobby (host notification).
    LobbyParticipantJoined { id: String, username: String },
    /// A waiting participant left the lobby.
    LobbyParticipantLeft { id: String },
    /// Entry was denied by the host (participant notification).
    LobbyDenied,
```

**Step 3: Build to check for compilation errors**

Run: `cargo build -p visio-core 2>&1 | head -40`

This will produce match-arm exhaustiveness errors in room.rs and other files that match on `ConnectionState` and `VisioEvent`. That's expected — we'll fix them in subsequent tasks.

**Step 4: Fix exhaustive matches in room.rs**

In `crates/visio-core/src/room.rs`, find the `set_connection_state` method or any match on `ConnectionState` and add the `WaitingForHost` arm. The `set_connection_state` method just stores and emits — no special handling needed.

**Step 5: Build and verify it compiles**

Run: `cargo build -p visio-core`
Expected: SUCCESS

**Step 6: Commit**

```bash
git add crates/visio-core/src/events.rs crates/visio-core/src/room.rs
git commit -m "feat(core): add WaitingForHost state and lobby events"
```

---

### Task 3: Create LobbyService in visio-core

New module that handles all Meet API lobby interactions.

**Files:**
- Create: `crates/visio-core/src/lobby.rs`
- Modify: `crates/visio-core/src/lib.rs:6-16` (add `pub mod lobby`)
- Modify: `crates/visio-core/src/lib.rs:18-31` (add re-exports)

**Step 1: Write the lobby service**

Create `crates/visio-core/src/lobby.rs`:

```rust
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use serde::{Deserialize, Serialize};

use crate::auth::AuthService;
use crate::errors::VisioError;

/// Status returned by the Meet lobby API.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LobbyStatus {
    Waiting,
    Accepted,
    Denied,
    #[serde(other)]
    Unknown,
}

/// A participant waiting in the lobby (from host's perspective).
#[derive(Debug, Clone, Deserialize)]
pub struct WaitingParticipant {
    pub id: String,
    pub username: String,
}

/// Response from POST /request-entry/.
#[derive(Debug, Clone, Deserialize)]
struct RequestEntryResponse {
    status: LobbyStatus,
    id: Option<String>,
    livekit: Option<RequestEntryLiveKit>,
}

#[derive(Debug, Clone, Deserialize)]
struct RequestEntryLiveKit {
    url: String,
    token: String,
}

/// Result of a lobby poll — either still waiting, accepted with token, or denied.
#[derive(Debug, Clone)]
pub enum LobbyPollResult {
    Waiting,
    Accepted { livekit_url: String, token: String },
    Denied,
}

pub struct LobbyService;

impl LobbyService {
    /// Request entry to a trusted room's lobby.
    ///
    /// Returns (lobby_participant_id, lobby_cookie, poll_result).
    /// The lobby_cookie must be stored and sent on subsequent polls.
    pub async fn request_entry(
        meet_url: &str,
        username: &str,
    ) -> Result<(String, Option<String>, LobbyPollResult), VisioError> {
        let (instance, slug) = AuthService::parse_meet_url(meet_url)?;
        let api_url = format!("https://{}/api/v1.0/rooms/{}/request-entry/", instance, slug);

        tracing::info!("requesting lobby entry: {}", api_url);

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| VisioError::Http(e.to_string()))?;

        let body = serde_json::json!({ "username": username });

        let resp = client
            .post(&api_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| VisioError::Http(e.to_string()))?;

        // Extract lobby cookie from Set-Cookie header
        let lobby_cookie = resp
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .find_map(|v| {
                let s = v.to_str().ok()?;
                if s.contains("lobby_") || s.contains("meet_") {
                    // Return the full cookie value for re-sending
                    Some(s.split(';').next()?.to_string())
                } else {
                    None
                }
            });

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(VisioError::Room(format!(
                "lobby request-entry failed ({status}): {body}"
            )));
        }

        let body_text = resp
            .text()
            .await
            .map_err(|e| VisioError::Http(e.to_string()))?;

        tracing::info!("lobby request-entry response: {}", body_text);

        let data: RequestEntryResponse = serde_json::from_str(&body_text)
            .map_err(|e| VisioError::Room(format!("invalid lobby response: {e} — {body_text}")))?;

        let participant_id = data.id.unwrap_or_default();

        let result = match data.status {
            LobbyStatus::Accepted => {
                if let Some(lk) = data.livekit {
                    let livekit_url = lk.url
                        .replace("https://", "wss://")
                        .replace("http://", "ws://");
                    LobbyPollResult::Accepted {
                        livekit_url,
                        token: lk.token,
                    }
                } else {
                    LobbyPollResult::Waiting
                }
            }
            LobbyStatus::Denied => LobbyPollResult::Denied,
            _ => LobbyPollResult::Waiting,
        };

        Ok((participant_id, lobby_cookie, result))
    }

    /// Poll the lobby by re-calling request-entry with the lobby cookie.
    /// This refreshes the server-side timeout and checks for status changes.
    pub async fn poll_entry(
        meet_url: &str,
        username: &str,
        lobby_cookie: Option<&str>,
    ) -> Result<LobbyPollResult, VisioError> {
        let (instance, slug) = AuthService::parse_meet_url(meet_url)?;
        let api_url = format!("https://{}/api/v1.0/rooms/{}/request-entry/", instance, slug);

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| VisioError::Http(e.to_string()))?;

        let body = serde_json::json!({ "username": username });

        let mut request = client.post(&api_url).json(&body);
        if let Some(cookie) = lobby_cookie {
            request = request.header(COOKIE, cookie);
        }

        let resp = request
            .send()
            .await
            .map_err(|e| VisioError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(VisioError::Room(format!(
                "lobby poll failed ({status}): {body}"
            )));
        }

        let body_text = resp
            .text()
            .await
            .map_err(|e| VisioError::Http(e.to_string()))?;

        tracing::debug!("lobby poll response: {}", body_text);

        let data: RequestEntryResponse = serde_json::from_str(&body_text)
            .map_err(|e| VisioError::Room(format!("invalid lobby poll response: {e}")))?;

        match data.status {
            LobbyStatus::Accepted => {
                if let Some(lk) = data.livekit {
                    let livekit_url = lk.url
                        .replace("https://", "wss://")
                        .replace("http://", "ws://");
                    Ok(LobbyPollResult::Accepted {
                        livekit_url,
                        token: lk.token,
                    })
                } else {
                    Ok(LobbyPollResult::Waiting)
                }
            }
            LobbyStatus::Denied => Ok(LobbyPollResult::Denied),
            _ => Ok(LobbyPollResult::Waiting),
        }
    }

    /// List participants waiting in the lobby (host only, requires auth).
    pub async fn list_waiting(
        meet_url: &str,
        session_cookie: &str,
    ) -> Result<Vec<WaitingParticipant>, VisioError> {
        let (instance, slug) = AuthService::parse_meet_url(meet_url)?;
        let api_url = format!(
            "https://{}/api/v1.0/rooms/{}/waiting-participants/",
            instance, slug
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(&api_url)
            .header(COOKIE, format!("sessionid={}", session_cookie))
            .send()
            .await
            .map_err(|e| VisioError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(VisioError::Room(format!(
                "list waiting participants failed ({status}): {body}"
            )));
        }

        let body_text = resp
            .text()
            .await
            .map_err(|e| VisioError::Http(e.to_string()))?;

        tracing::debug!("waiting-participants response: {}", body_text);

        serde_json::from_str(&body_text)
            .map_err(|e| VisioError::Room(format!("invalid waiting-participants response: {e}")))
    }

    /// Accept or reject a waiting participant (host only, requires auth).
    pub async fn handle_entry(
        meet_url: &str,
        session_cookie: &str,
        participant_id: &str,
        allow: bool,
    ) -> Result<(), VisioError> {
        let (instance, slug) = AuthService::parse_meet_url(meet_url)?;
        let api_url = format!("https://{}/api/v1.0/rooms/{}/enter/", instance, slug);

        // Need CSRF token for POST
        use rand::Rng;
        let csrf_bytes: [u8; 32] = rand::thread_rng().r#gen();
        let csrf_token: String = csrf_bytes.iter().map(|b| format!("{:02x}", b)).collect();

        let cookie_header = format!(
            "sessionid={}; csrftoken={}",
            session_cookie, csrf_token
        );

        let body = serde_json::json!({
            "participant_id": participant_id,
            "allow_entry": allow,
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&api_url)
            .header(COOKIE, &cookie_header)
            .header("X-CSRFToken", &csrf_token)
            .header("Referer", format!("https://{}/", instance))
            .json(&body)
            .send()
            .await
            .map_err(|e| VisioError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(VisioError::Room(format!(
                "handle entry failed ({status}): {body}"
            )));
        }

        tracing::info!(
            "lobby entry handled: participant={}, allow={}",
            participant_id,
            allow
        );

        Ok(())
    }
}
```

**Step 2: Register the module and add re-exports**

In `crates/visio-core/src/lib.rs`, add after `pub mod hand_raise;`:

```rust
pub mod lobby;
```

And add to the re-exports:

```rust
pub use lobby::{LobbyPollResult, LobbyService, LobbyStatus, WaitingParticipant};
```

**Step 3: Make `AuthService::parse_meet_url` public**

In `crates/visio-core/src/auth.rs:137`, change:

```rust
    fn parse_meet_url(url: &str) -> Result<(String, String), VisioError> {
```

to:

```rust
    pub fn parse_meet_url(url: &str) -> Result<(String, String), VisioError> {
```

**Step 4: Build**

Run: `cargo build -p visio-core`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add crates/visio-core/src/lobby.rs crates/visio-core/src/lib.rs crates/visio-core/src/auth.rs
git commit -m "feat(core): add LobbyService for waiting room API"
```

---

### Task 4: Integrate lobby flow into RoomManager

Modify connect() to detect lobby state and start polling. Add host-side lobby management methods.

**Files:**
- Modify: `crates/visio-core/src/room.rs:22-39` (add lobby fields to RoomManager)
- Modify: `crates/visio-core/src/room.rs:47-65` (update new() constructor)
- Modify: `crates/visio-core/src/room.rs:171-187` (modify connect() for lobby detection)
- Modify: `crates/visio-core/src/room.rs` (add lobby management methods)

**Step 1: Add lobby state fields to RoomManager**

Add these fields to the `RoomManager` struct (after `last_username`):

```rust
    /// Lobby cookie for anonymous participant polling.
    lobby_cookie: Arc<Mutex<Option<String>>>,
    /// Session cookie stored from connect() for lobby host operations.
    session_cookie: Arc<Mutex<Option<String>>>,
    /// Flag to cancel lobby polling.
    lobby_cancel: Arc<tokio::sync::Notify>,
```

Initialize them in `new()`:

```rust
    lobby_cookie: Arc::new(Mutex::new(None)),
    session_cookie: Arc::new(Mutex::new(None)),
    lobby_cancel: Arc::new(tokio::sync::Notify::new()),
```

**Step 2: Modify connect() to handle lobby**

Replace the current `connect()` method body. The key change: when `request_token()` returns the "waiting for host approval" error, switch to lobby mode instead of failing.

```rust
    pub async fn connect(
        &self,
        meet_url: &str,
        username: Option<&str>,
        session_cookie: Option<&str>,
    ) -> Result<(), VisioError> {
        // Store connection info for potential reconnection
        *self.last_meet_url.lock().await = Some(meet_url.to_string());
        *self.last_username.lock().await = username.map(|s| s.to_string());
        *self.session_cookie.lock().await = session_cookie.map(|s| s.to_string());

        self.set_connection_state(ConnectionState::Connecting).await;

        match crate::auth::AuthService::request_token(meet_url, username, session_cookie).await {
            Ok(token_info) => {
                // Direct connection (public room, or authenticated on trusted room)
                self.connect_with_token(&token_info.livekit_url, &token_info.token)
                    .await
            }
            Err(VisioError::Auth(msg)) if msg.contains("waiting for host approval") => {
                // Trusted room, anonymous user → enter lobby
                tracing::info!("room requires host approval, entering lobby");
                let display_name = username.unwrap_or("Anonymous");
                self.enter_lobby(meet_url, display_name).await
            }
            Err(e) => Err(e),
        }
    }
```

**Step 3: Add `enter_lobby()` and `cancel_lobby()` methods**

Add these methods to `impl RoomManager`:

```rust
    /// Enter the lobby and start polling for host approval.
    async fn enter_lobby(
        &self,
        meet_url: &str,
        username: &str,
    ) -> Result<(), VisioError> {
        let (participant_id, cookie, initial_result) =
            crate::lobby::LobbyService::request_entry(meet_url, username).await?;

        tracing::info!("lobby entry requested, participant_id={participant_id}");

        // Store lobby cookie for subsequent polls
        *self.lobby_cookie.lock().await = cookie;

        match initial_result {
            crate::lobby::LobbyPollResult::Accepted { livekit_url, token } => {
                // Bypassed lobby (authenticated user)
                self.connect_with_token(&livekit_url, &token).await
            }
            crate::lobby::LobbyPollResult::Denied => {
                self.set_connection_state(ConnectionState::Disconnected).await;
                self.emitter.emit(VisioEvent::LobbyDenied);
                Ok(())
            }
            crate::lobby::LobbyPollResult::Waiting => {
                self.set_connection_state(ConnectionState::WaitingForHost).await;
                // Start polling in background
                let meet_url = meet_url.to_string();
                let username = username.to_string();
                let lobby_cookie = self.lobby_cookie.clone();
                let emitter = self.emitter.clone();
                let connection_state = self.connection_state.clone();
                let cancel = self.lobby_cancel.clone();
                let room_self = LobbyPollContext {
                    last_meet_url: self.last_meet_url.clone(),
                    last_username: self.last_username.clone(),
                    session_cookie: self.session_cookie.clone(),
                };

                // Clone self references needed for connect_with_token
                let room = self.room.clone();
                let participants = self.participants.clone();
                let subscribed_tracks = self.subscribed_tracks.clone();
                let messages = self.messages.clone();
                let playout_buffer = self.playout_buffer.clone();
                let hand_raise = self.hand_raise.clone();
                let camera_enabled = self.camera_enabled.clone();

                tokio::spawn(async move {
                    Self::lobby_poll_loop(
                        meet_url,
                        username,
                        lobby_cookie,
                        emitter,
                        connection_state,
                        cancel,
                        room,
                        participants,
                        subscribed_tracks,
                        messages,
                        playout_buffer,
                        hand_raise,
                        camera_enabled,
                        room_self,
                    )
                    .await;
                });

                Ok(())
            }
        }
    }

    /// Cancel lobby polling (called on disconnect or when user cancels).
    pub async fn cancel_lobby(&self) {
        self.lobby_cancel.notify_one();
        *self.lobby_cookie.lock().await = None;
    }
```

**Step 4: Add the poll loop**

Add a helper struct and the polling loop:

```rust
struct LobbyPollContext {
    last_meet_url: Arc<Mutex<Option<String>>>,
    last_username: Arc<Mutex<Option<String>>>,
    session_cookie: Arc<Mutex<Option<String>>>,
}

impl RoomManager {
    async fn lobby_poll_loop(
        meet_url: String,
        username: String,
        lobby_cookie: Arc<Mutex<Option<String>>>,
        emitter: EventEmitter,
        connection_state: Arc<Mutex<ConnectionState>>,
        cancel: Arc<tokio::sync::Notify>,
        room: Arc<Mutex<Option<Arc<Room>>>>,
        participants: Arc<Mutex<ParticipantManager>>,
        subscribed_tracks: Arc<Mutex<HashMap<String, RemoteVideoTrack>>>,
        messages: MessageStore,
        playout_buffer: Arc<AudioPlayoutBuffer>,
        hand_raise: Arc<Mutex<Option<HandRaiseManager>>>,
        camera_enabled: Arc<Mutex<bool>>,
        ctx: LobbyPollContext,
    ) {
        loop {
            tokio::select! {
                _ = cancel.notified() => {
                    tracing::info!("lobby polling cancelled");
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => {
                    let cookie = lobby_cookie.lock().await.clone();
                    match crate::lobby::LobbyService::poll_entry(
                        &meet_url,
                        &username,
                        cookie.as_deref(),
                    ).await {
                        Ok(crate::lobby::LobbyPollResult::Accepted { livekit_url, token }) => {
                            tracing::info!("lobby: accepted by host, connecting to LiveKit");
                            // Connect to the room
                            *connection_state.lock().await = ConnectionState::Connecting;
                            emitter.emit(VisioEvent::ConnectionStateChanged(ConnectionState::Connecting));

                            let options = RoomOptions {
                                auto_subscribe: true,
                                adaptive_stream: true,
                                dynacast: true,
                                ..Default::default()
                            };

                            match Room::connect(&livekit_url, &token, options).await {
                                Ok((new_room, events)) => {
                                    let new_room = Arc::new(new_room);

                                    // Store local participant SID
                                    {
                                        let local = new_room.local_participant();
                                        let mut pm = participants.lock().await;
                                        pm.set_local_sid(local.sid().to_string());
                                    }

                                    // Seed existing remote participants
                                    {
                                        let mut pm = participants.lock().await;
                                        for (_, participant) in new_room.remote_participants() {
                                            let info = Self::remote_participant_to_info(&participant);
                                            pm.add_participant(info.clone());
                                            emitter.emit(VisioEvent::ParticipantJoined(info));
                                        }
                                    }

                                    *room.lock().await = Some(new_room.clone());

                                    // Initialize HandRaiseManager
                                    {
                                        let hm = HandRaiseManager::new(new_room.clone(), emitter.clone());
                                        *hand_raise.lock().await = Some(hm);
                                    }

                                    *connection_state.lock().await = ConnectionState::Connected;
                                    emitter.emit(VisioEvent::ConnectionStateChanged(ConnectionState::Connected));

                                    // Spawn event loop
                                    let ev_emitter = emitter.clone();
                                    let ev_participants = participants.clone();
                                    let ev_conn_state = connection_state.clone();
                                    let ev_room = room.clone();
                                    let ev_tracks = subscribed_tracks.clone();
                                    let ev_messages = messages.clone();
                                    let ev_playout = playout_buffer.clone();
                                    let ev_hand_raise = hand_raise.clone();
                                    let ev_last_url = ctx.last_meet_url.clone();

                                    tokio::spawn(async move {
                                        Self::event_loop(
                                            events,
                                            ev_emitter,
                                            ev_participants,
                                            ev_conn_state,
                                            ev_room,
                                            ev_tracks,
                                            ev_messages,
                                            ev_playout,
                                            ev_hand_raise,
                                            ev_last_url,
                                        )
                                        .await;
                                    });
                                }
                                Err(e) => {
                                    tracing::error!("lobby: failed to connect after acceptance: {e}");
                                    *connection_state.lock().await = ConnectionState::Disconnected;
                                    emitter.emit(VisioEvent::ConnectionStateChanged(ConnectionState::Disconnected));
                                }
                            }
                            break;
                        }
                        Ok(crate::lobby::LobbyPollResult::Denied) => {
                            tracing::info!("lobby: denied by host");
                            *connection_state.lock().await = ConnectionState::Disconnected;
                            emitter.emit(VisioEvent::LobbyDenied);
                            emitter.emit(VisioEvent::ConnectionStateChanged(ConnectionState::Disconnected));
                            break;
                        }
                        Ok(crate::lobby::LobbyPollResult::Waiting) => {
                            // Still waiting, continue polling
                        }
                        Err(e) => {
                            tracing::warn!("lobby poll error: {e}, will retry");
                        }
                    }
                }
            }
        }
    }
}
```

**Step 5: Add host-side lobby methods**

Add these public methods to `impl RoomManager`:

```rust
    /// List participants waiting in the lobby (host only).
    pub async fn list_waiting_participants(
        &self,
    ) -> Result<Vec<crate::lobby::WaitingParticipant>, VisioError> {
        let meet_url = self.last_meet_url.lock().await.clone()
            .ok_or_else(|| VisioError::Room("not connected".to_string()))?;
        let cookie = self.session_cookie.lock().await.clone()
            .ok_or_else(|| VisioError::Room("not authenticated".to_string()))?;

        crate::lobby::LobbyService::list_waiting(&meet_url, &cookie).await
    }

    /// Accept or reject a waiting participant (host only).
    pub async fn handle_lobby_entry(
        &self,
        participant_id: &str,
        allow: bool,
    ) -> Result<(), VisioError> {
        let meet_url = self.last_meet_url.lock().await.clone()
            .ok_or_else(|| VisioError::Room("not connected".to_string()))?;
        let cookie = self.session_cookie.lock().await.clone()
            .ok_or_else(|| VisioError::Room("not authenticated".to_string()))?;

        crate::lobby::LobbyService::handle_entry(&meet_url, &cookie, participant_id, allow).await
    }
```

**Step 6: Update disconnect() to cancel lobby polling**

In `disconnect()`, add at the beginning:

```rust
    pub async fn disconnect(&self) {
        // Cancel any pending lobby polling
        self.lobby_cancel.notify_one();
        *self.lobby_cookie.lock().await = None;

        // Clear reconnection info BEFORE closing ...
        // (rest of existing code)
    }
```

**Step 7: Build and run tests**

Run: `cargo build -p visio-core && cargo test -p visio-core`
Expected: All existing tests pass, no compile errors

**Step 8: Commit**

```bash
git add crates/visio-core/src/room.rs
git commit -m "feat(core): integrate lobby flow into RoomManager connect()"
```

---

### Task 5: Handle lobby data channel notifications for host

When a participant enters the lobby, the Meet server sends a notification to connected hosts via LiveKit data channels. Detect these in the event loop and emit `LobbyParticipantJoined`.

**Files:**
- Modify: `crates/visio-core/src/room.rs` (event loop, around lines 695-834 where data channels are handled)

**Step 1: Add lobby notification handling to event loop**

In the `event_loop` method, inside the `RoomEvent::DataReceived` match arm (or `TextStreamOpened`), add detection for lobby-type messages. The Meet server sends notifications with a specific topic.

Find the data channel handling section and add before or after the chat handling:

```rust
                // Handle lobby notification from Meet server
                RoomEvent::DataReceived { payload, topic, .. } => {
                    if let Some(ref t) = topic {
                        if t.contains("lobby") || t.contains("waiting") {
                            // Parse lobby notification
                            if let Ok(text) = std::str::from_utf8(&payload) {
                                tracing::info!("lobby notification received: {}", text);
                                // The notification tells the host someone is waiting.
                                // We don't parse the payload — just emit a generic event
                                // and let the UI poll for the full list.
                                if let Ok(data) = serde_json::from_str::<serde_json::Value>(text) {
                                    let id = data.get("id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let username = data.get("username")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Unknown")
                                        .to_string();
                                    emitter.emit(VisioEvent::LobbyParticipantJoined { id, username });
                                }
                            }
                            continue;
                        }
                    }
                    // ... existing chat/data handling
                }
```

Note: The exact topic name used by the Meet server needs to be verified against the actual backend. If the topic doesn't match, the host can still poll `waiting-participants` periodically. The data channel notification is an optimization for immediate feedback.

**Step 2: Build and test**

Run: `cargo build -p visio-core && cargo test -p visio-core`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add crates/visio-core/src/room.rs
git commit -m "feat(core): handle lobby data channel notifications for host"
```

---

### Task 6: Expose lobby to UniFFI (Android/iOS)

Add lobby types and methods to the UniFFI interface definition and FFI bridge.

**Files:**
- Modify: `crates/visio-ffi/src/visio.udl:5-11` (ConnectionState)
- Modify: `crates/visio-ffi/src/visio.udl:69-84` (VisioEvent)
- Modify: `crates/visio-ffi/src/visio.udl:86-94` (VisioError — add AuthRequired)
- Modify: `crates/visio-ffi/src/visio.udl:122-209` (VisioClient — add lobby methods)
- Modify: `crates/visio-ffi/src/lib.rs:91-108` (ConnectionState FFI type)
- Modify: `crates/visio-ffi/src/lib.rs:284-343` (VisioEvent FFI type + From impl)
- Modify: `crates/visio-ffi/src/lib.rs` (add lobby methods to VisioClient impl)

**Step 1: Update visio.udl**

Add `WaitingForHost` to ConnectionState:
```
[Enum]
interface ConnectionState {
    Disconnected();
    Connecting();
    Connected();
    Reconnecting(u32 attempt);
    WaitingForHost();
};
```

Add lobby events to VisioEvent:
```
[Enum]
interface VisioEvent {
    // ... existing variants ...
    LobbyParticipantJoined(string id, string username);
    LobbyParticipantLeft(string id);
    LobbyDenied();
};
```

Add WaitingParticipant dictionary and lobby methods:
```
dictionary WaitingParticipant {
    string id;
    string username;
};

// Add to VisioClient interface:
    [Throws=VisioError]
    sequence<WaitingParticipant> list_waiting_participants();

    [Throws=VisioError]
    void admit_participant(string participant_id);

    [Throws=VisioError]
    void deny_participant(string participant_id);

    void cancel_lobby();
```

**Step 2: Update FFI types in lib.rs**

Add `WaitingForHost` to the `ConnectionState` enum and its `From` impl:

```rust
#[derive(Debug, Clone)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32 },
    WaitingForHost,
}

impl From<CoreConnectionState> for ConnectionState {
    fn from(s: CoreConnectionState) -> Self {
        match s {
            CoreConnectionState::Disconnected => Self::Disconnected,
            CoreConnectionState::Connecting => Self::Connecting,
            CoreConnectionState::Connected => Self::Connected,
            CoreConnectionState::Reconnecting { attempt } => Self::Reconnecting { attempt },
            CoreConnectionState::WaitingForHost => Self::WaitingForHost,
        }
    }
}
```

Add lobby events to `VisioEvent` enum and its `From` impl:

```rust
// Add to VisioEvent enum:
    LobbyParticipantJoined { id: String, username: String },
    LobbyParticipantLeft { id: String },
    LobbyDenied,

// Add to From<CoreVisioEvent> impl:
    CoreVisioEvent::LobbyParticipantJoined { id, username } => {
        Self::LobbyParticipantJoined { id, username }
    }
    CoreVisioEvent::LobbyParticipantLeft { id } => {
        Self::LobbyParticipantLeft { id }
    }
    CoreVisioEvent::LobbyDenied => Self::LobbyDenied,
```

Add `WaitingParticipant` FFI type:

```rust
#[derive(Debug, Clone)]
pub struct WaitingParticipant {
    pub id: String,
    pub username: String,
}

impl From<visio_core::WaitingParticipant> for WaitingParticipant {
    fn from(w: visio_core::WaitingParticipant) -> Self {
        Self {
            id: w.id,
            username: w.username,
        }
    }
}
```

**Step 3: Add lobby methods to VisioClient impl**

```rust
    pub fn list_waiting_participants(&self) -> Result<Vec<WaitingParticipant>, VisioError> {
        self.rt
            .block_on(self.room_manager.list_waiting_participants())
            .map(|v| v.into_iter().map(Into::into).collect())
            .map_err(Into::into)
    }

    pub fn admit_participant(&self, participant_id: String) -> Result<(), VisioError> {
        self.rt
            .block_on(self.room_manager.handle_lobby_entry(&participant_id, true))
            .map_err(Into::into)
    }

    pub fn deny_participant(&self, participant_id: String) -> Result<(), VisioError> {
        self.rt
            .block_on(self.room_manager.handle_lobby_entry(&participant_id, false))
            .map_err(Into::into)
    }

    pub fn cancel_lobby(&self) {
        self.rt.block_on(self.room_manager.cancel_lobby());
    }
```

**Step 4: Build**

Run: `cargo build -p visio-ffi`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add crates/visio-ffi/src/visio.udl crates/visio-ffi/src/lib.rs
git commit -m "feat(ffi): expose lobby/waiting room to UniFFI"
```

---

### Task 7: Desktop — Add lobby Tauri commands and event handling

**Files:**
- Modify: `crates/visio-desktop/src/lib.rs:80-239` (DesktopEventListener)
- Modify: `crates/visio-desktop/src/lib.rs:275-288` (connect command)
- Modify: `crates/visio-desktop/src/lib.rs:908-939` (invoke_handler)

**Step 1: Handle new events in DesktopEventListener**

Add to the `on_event` match in `DesktopEventListener` (around line 236, before the closing `}`):

```rust
            VisioEvent::ConnectionStateChanged(state) => {
                let name = match &state {
                    visio_core::ConnectionState::Disconnected => "disconnected",
                    visio_core::ConnectionState::Connecting => "connecting",
                    visio_core::ConnectionState::Connected => "connected",
                    visio_core::ConnectionState::Reconnecting { .. } => "reconnecting",
                    visio_core::ConnectionState::WaitingForHost => "waiting_for_host",
                };
                // ... rest is same
            }
            // Add new event handlers:
            VisioEvent::LobbyParticipantJoined { id, username } => {
                tracing::info!("lobby participant joined: {username} (id={id})");
                if let Some(app) = APP_HANDLE.get() {
                    let _ = app.emit(
                        "lobby-participant-joined",
                        serde_json::json!({ "id": id, "username": username }),
                    );
                }
            }
            VisioEvent::LobbyParticipantLeft { id } => {
                if let Some(app) = APP_HANDLE.get() {
                    let _ = app.emit("lobby-participant-left", &id);
                }
            }
            VisioEvent::LobbyDenied => {
                if let Some(app) = APP_HANDLE.get() {
                    let _ = app.emit("lobby-denied", ());
                }
            }
```

Also update the `get_connection_state` command to handle the new variant:
```rust
        visio_core::ConnectionState::WaitingForHost => "waiting_for_host",
```

**Step 2: Add lobby Tauri commands**

```rust
#[tauri::command]
async fn list_waiting_participants(
    state: tauri::State<'_, VisioState>,
) -> Result<serde_json::Value, String> {
    let room = state.room.lock().await;
    let participants = room
        .list_waiting_participants()
        .await
        .map_err(|e| e.to_string())?;
    let json: Vec<_> = participants
        .iter()
        .map(|p| serde_json::json!({ "id": p.id, "username": p.username }))
        .collect();
    Ok(serde_json::json!(json))
}

#[tauri::command]
async fn admit_participant(
    state: tauri::State<'_, VisioState>,
    participant_id: String,
) -> Result<(), String> {
    let room = state.room.lock().await;
    room.handle_lobby_entry(&participant_id, true)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn deny_participant(
    state: tauri::State<'_, VisioState>,
    participant_id: String,
) -> Result<(), String> {
    let room = state.room.lock().await;
    room.handle_lobby_entry(&participant_id, false)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn cancel_lobby(state: tauri::State<'_, VisioState>) -> Result<(), String> {
    let room = state.room.lock().await;
    room.cancel_lobby().await;
    Ok(())
}
```

**Step 3: Register new commands**

Add to `invoke_handler`:

```rust
    list_waiting_participants,
    admit_participant,
    deny_participant,
    cancel_lobby,
```

**Step 4: Build**

Run: `cargo build -p visio-desktop`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add crates/visio-desktop/src/lib.rs
git commit -m "feat(desktop): add lobby Tauri commands and event handling"
```

---

### Task 8: Desktop — Frontend waiting screen and host UI

**Files:**
- Modify: `crates/visio-desktop/frontend/src/App.tsx` (add waiting screen state + host lobby panel)
- Modify: `crates/visio-desktop/frontend/src/App.css` (waiting screen styles)

**Step 1: Add waiting screen UI in App.tsx**

In the main App component, detect `connectionState === "waiting_for_host"` and show a waiting screen instead of the call view. This should be added to the state machine that handles `connectionState`.

Add a `WaitingScreen` component:

```tsx
function WaitingScreen({ onCancel, t }: { onCancel: () => void; t: (k: string) => string }) {
  return (
    <div className="waiting-screen">
      <div className="waiting-content">
        <div className="waiting-spinner" />
        <h2>{t("lobby.waiting")}</h2>
        <p>{t("lobby.waitingDesc")}</p>
        <button className="btn btn-secondary" onClick={onCancel}>
          {t("lobby.cancel")}
        </button>
      </div>
    </div>
  );
}
```

In the main render logic, add before the call view:

```tsx
if (connectionState === "waiting_for_host") {
  return (
    <WaitingScreen
      onCancel={async () => {
        await invoke("cancel_lobby");
        await invoke("disconnect");
        setConnectionState("disconnected");
      }}
      t={t}
    />
  );
}
```

**Step 2: Add lobby denied handling**

Listen for the `lobby-denied` event:

```tsx
useEffect(() => {
  const unlisten = listen("lobby-denied", () => {
    setConnectionState("disconnected");
    // Optionally show a toast/alert
    alert(t("lobby.denied"));
  });
  return () => { unlisten.then(f => f()); };
}, []);
```

**Step 3: Add host lobby panel in participants view**

In the participants panel/sidebar, add a "Waiting room" section that shows when there are waiting participants.

Add state for waiting participants:

```tsx
const [waitingParticipants, setWaitingParticipants] = useState<Array<{id: string, username: string}>>([]);
```

Listen for lobby events:

```tsx
useEffect(() => {
  const unsub1 = listen("lobby-participant-joined", (e: any) => {
    setWaitingParticipants(prev => {
      if (prev.some(p => p.id === e.payload.id)) return prev;
      return [...prev, e.payload];
    });
  });
  const unsub2 = listen("lobby-participant-left", (e: any) => {
    setWaitingParticipants(prev => prev.filter(p => p.id !== e.payload));
  });
  return () => {
    unsub1.then(f => f());
    unsub2.then(f => f());
  };
}, []);
```

Add lobby UI in the participants sidebar:

```tsx
{waitingParticipants.length > 0 && (
  <div className="lobby-section">
    <div className="lobby-header">
      <h4>{t("lobby.waitingParticipants")} ({waitingParticipants.length})</h4>
      <button
        className="btn btn-sm"
        onClick={async () => {
          for (const p of waitingParticipants) {
            await invoke("admit_participant", { participantId: p.id });
          }
          setWaitingParticipants([]);
        }}
      >
        {t("lobby.admitAll")}
      </button>
    </div>
    {waitingParticipants.map(p => (
      <div key={p.id} className="lobby-participant">
        <span>{p.username}</span>
        <div className="lobby-actions">
          <button
            className="btn btn-sm btn-primary"
            onClick={async () => {
              await invoke("admit_participant", { participantId: p.id });
              setWaitingParticipants(prev => prev.filter(x => x.id !== p.id));
            }}
          >
            {t("lobby.admit")}
          </button>
          <button
            className="btn btn-sm btn-danger"
            onClick={async () => {
              await invoke("deny_participant", { participantId: p.id });
              setWaitingParticipants(prev => prev.filter(x => x.id !== p.id));
            }}
          >
            {t("lobby.deny")}
          </button>
        </div>
      </div>
    ))}
  </div>
)}
```

**Step 4: Add CSS styles**

Add to `App.css`:

```css
/* Waiting screen */
.waiting-screen {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100vh;
  background: var(--bg);
}

.waiting-content {
  text-align: center;
  max-width: 400px;
  padding: 40px;
}

.waiting-spinner {
  width: 48px;
  height: 48px;
  border: 4px solid var(--border);
  border-top-color: var(--primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin: 0 auto 24px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.waiting-content h2 {
  margin: 0 0 8px;
  font-size: 1.25rem;
}

.waiting-content p {
  color: var(--text-secondary);
  margin: 0 0 24px;
}

/* Lobby section in participants */
.lobby-section {
  border-top: 1px solid var(--border);
  padding-top: 12px;
  margin-top: 12px;
}

.lobby-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.lobby-header h4 {
  margin: 0;
  font-size: 0.9rem;
}

.lobby-participant {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
}

.lobby-actions {
  display: flex;
  gap: 6px;
}

.btn-danger {
  background: #e74c3c;
  color: white;
  border: none;
}

.btn-danger:hover {
  background: #c0392b;
}
```

**Step 5: Build and verify**

Run: `cd crates/visio-desktop && cargo tauri dev`
Expected: App runs, waiting screen shows when connecting to a trusted room as anonymous

**Step 6: Commit**

```bash
git add crates/visio-desktop/frontend/src/App.tsx crates/visio-desktop/frontend/src/App.css
git commit -m "feat(desktop): add waiting screen and host lobby UI"
```

---

### Task 9: Android — Waiting screen and host lobby UI

**Files:**
- Modify: `android/app/src/main/kotlin/io/visio/mobile/VisioManager.kt` (add lobby state + methods)
- Modify: `android/app/src/main/kotlin/io/visio/mobile/ui/CallScreen.kt` (waiting screen + host UI)

**Step 1: Add lobby state to VisioManager**

In `VisioManager.kt`, add state for waiting room:

```kotlin
// In VisioManager class:
val waitingParticipants = MutableStateFlow<List<WaitingParticipant>>(emptyList())
val lobbyDenied = MutableStateFlow(false)
```

In the event listener `on_event`, add handling for new events:

```kotlin
is VisioEvent.LobbyParticipantJoined -> {
    val current = waitingParticipants.value.toMutableList()
    if (current.none { it.id == event.id }) {
        current.add(WaitingParticipant(event.id, event.username))
        waitingParticipants.value = current
    }
}
is VisioEvent.LobbyParticipantLeft -> {
    waitingParticipants.value = waitingParticipants.value.filter { it.id != event.id }
}
is VisioEvent.LobbyDenied -> {
    lobbyDenied.value = true
}
```

Add lobby methods:

```kotlin
fun admitParticipant(participantId: String) {
    try {
        client.admitParticipant(participantId)
        waitingParticipants.value = waitingParticipants.value.filter { it.id != participantId }
    } catch (e: Exception) {
        Log.e("VisioManager", "admit failed: ${e.message}")
    }
}

fun denyParticipant(participantId: String) {
    try {
        client.denyParticipant(participantId)
        waitingParticipants.value = waitingParticipants.value.filter { it.id != participantId }
    } catch (e: Exception) {
        Log.e("VisioManager", "deny failed: ${e.message}")
    }
}

fun cancelLobby() {
    client.cancelLobby()
}
```

**Step 2: Add waiting screen to CallScreen**

In `CallScreen.kt`, check `connectionState` for `WaitingForHost`:

```kotlin
val connectionState by VisioManager.connectionState.collectAsState()

when (connectionState) {
    is ConnectionState.WaitingForHost -> {
        // Show waiting screen
        WaitingScreen(
            onCancel = {
                VisioManager.cancelLobby()
                VisioManager.client.disconnect()
                navController.popBackStack()
            }
        )
    }
    // ... existing states
}
```

Add `WaitingScreen` composable:

```kotlin
@Composable
fun WaitingScreen(onCancel: () -> Unit) {
    val lang by VisioManager.language.collectAsState()

    Column(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        CircularProgressIndicator(
            modifier = Modifier.size(48.dp),
            color = MaterialTheme.colorScheme.primary
        )
        Spacer(modifier = Modifier.height(24.dp))
        Text(
            text = Strings.t("lobby.waiting", lang),
            style = MaterialTheme.typography.titleMedium
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = Strings.t("lobby.waitingDesc", lang),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.height(24.dp))
        OutlinedButton(onClick = onCancel) {
            Text(Strings.t("lobby.cancel", lang))
        }
    }
}
```

**Step 3: Add host lobby panel**

In the participants panel (or InCallSettingsSheet), add a "Waiting room" section:

```kotlin
val waitingParticipants by VisioManager.waitingParticipants.collectAsState()

if (waitingParticipants.isNotEmpty()) {
    Text(
        text = Strings.t("lobby.waitingParticipants", lang) + " (${waitingParticipants.size})",
        style = MaterialTheme.typography.titleSmall,
        modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
    )
    waitingParticipants.forEach { participant ->
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 4.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(participant.username, modifier = Modifier.weight(1f))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(
                    onClick = { VisioManager.admitParticipant(participant.id) },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.primary
                    )
                ) {
                    Text(Strings.t("lobby.admit", lang))
                }
                OutlinedButton(
                    onClick = { VisioManager.denyParticipant(participant.id) }
                ) {
                    Text(Strings.t("lobby.deny", lang))
                }
            }
        }
    }
}
```

**Step 4: Handle lobby denied**

In CallScreen, observe `lobbyDenied`:

```kotlin
val lobbyDenied by VisioManager.lobbyDenied.collectAsState()

LaunchedEffect(lobbyDenied) {
    if (lobbyDenied) {
        // Show toast and navigate back
        Toast.makeText(context, Strings.t("lobby.denied", lang), Toast.LENGTH_LONG).show()
        VisioManager.lobbyDenied.value = false
        navController.popBackStack()
    }
}
```

**Step 5: Build Android native libs**

Run: `cargo ndk -t arm64-v8a build -p visio-ffi -p visio-video --release`
Then regenerate UniFFI bindings:
Run: `scripts/generate-bindings.sh kotlin`

**Step 6: Commit**

```bash
git add android/app/src/main/kotlin/io/visio/mobile/VisioManager.kt \
        android/app/src/main/kotlin/io/visio/mobile/ui/CallScreen.kt
git commit -m "feat(android): add waiting screen and host lobby UI"
```

---

### Task 10: iOS — Waiting screen and host lobby UI

**Files:**
- Modify: `ios/VisioMobile/VisioManager.swift` (add lobby state + methods)
- Modify: `ios/VisioMobile/Views/CallView.swift` (waiting screen + host UI)

**Step 1: Add lobby state to VisioManager**

In `VisioManager.swift`, add published properties:

```swift
@Published var waitingParticipants: [WaitingParticipant] = []
@Published var lobbyDenied = false
```

In the event handler, add:

```swift
case let .lobbyParticipantJoined(id, username):
    DispatchQueue.main.async {
        if !self.waitingParticipants.contains(where: { $0.id == id }) {
            self.waitingParticipants.append(WaitingParticipant(id: id, username: username))
        }
    }
case let .lobbyParticipantLeft(id):
    DispatchQueue.main.async {
        self.waitingParticipants.removeAll { $0.id == id }
    }
case .lobbyDenied:
    DispatchQueue.main.async {
        self.lobbyDenied = true
    }
```

Add methods:

```swift
func admitParticipant(_ id: String) {
    do {
        try client.admitParticipant(participantId: id)
        waitingParticipants.removeAll { $0.id == id }
    } catch {
        NSLog("VisioManager: admit failed: \(error)")
    }
}

func denyParticipant(_ id: String) {
    do {
        try client.denyParticipant(participantId: id)
        waitingParticipants.removeAll { $0.id == id }
    } catch {
        NSLog("VisioManager: deny failed: \(error)")
    }
}

func cancelLobby() {
    client.cancelLobby()
}
```

**Step 2: Add waiting screen to CallView**

In `CallView.swift`, add waiting state handling:

```swift
if visioManager.connectionState == .waitingForHost {
    WaitingRoomView(
        onCancel: {
            visioManager.cancelLobby()
            visioManager.disconnect()
            presentationMode.wrappedValue.dismiss()
        }
    )
} else {
    // existing call view content
}
```

Create `WaitingRoomView`:

```swift
struct WaitingRoomView: View {
    let onCancel: () -> Void
    @EnvironmentObject var visioManager: VisioManager

    var body: some View {
        VStack(spacing: 24) {
            Spacer()
            ProgressView()
                .scaleEffect(1.5)
            Text(Strings.t("lobby.waiting", lang: visioManager.language))
                .font(.title2)
            Text(Strings.t("lobby.waitingDesc", lang: visioManager.language))
                .foregroundColor(.secondary)
            Button(action: onCancel) {
                Text(Strings.t("lobby.cancel", lang: visioManager.language))
            }
            .buttonStyle(.bordered)
            Spacer()
        }
        .padding()
    }
}
```

**Step 3: Add host lobby panel**

In the participants section (or InCallSettingsSheet), add:

```swift
if !visioManager.waitingParticipants.isEmpty {
    Section(header: Text("\(Strings.t("lobby.waitingParticipants", lang: lang)) (\(visioManager.waitingParticipants.count))")) {
        ForEach(visioManager.waitingParticipants, id: \.id) { participant in
            HStack {
                Text(participant.username)
                Spacer()
                Button(Strings.t("lobby.admit", lang: lang)) {
                    visioManager.admitParticipant(participant.id)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.small)

                Button(Strings.t("lobby.deny", lang: lang)) {
                    visioManager.denyParticipant(participant.id)
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
                .tint(.red)
            }
        }
    }
}
```

**Step 4: Handle lobby denied**

In CallView, observe `lobbyDenied`:

```swift
.onChange(of: visioManager.lobbyDenied) { denied in
    if denied {
        visioManager.lobbyDenied = false
        presentationMode.wrappedValue.dismiss()
    }
}
```

**Step 5: Build iOS native libs**

Run: `scripts/generate-bindings.sh swift`
Run: `cargo build -p visio-ffi -p visio-video --target aarch64-apple-ios --release`

**Step 6: Commit**

```bash
git add ios/VisioMobile/VisioManager.swift \
        ios/VisioMobile/Views/CallView.swift
git commit -m "feat(ios): add waiting screen and host lobby UI"
```

---

### Task 11: Unit tests for LobbyService

**Files:**
- Modify: `crates/visio-core/src/lobby.rs` (add tests module)

**Step 1: Add unit tests**

Add to end of `crates/visio-core/src/lobby.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lobby_status_waiting() {
        let json = r#"{"status": "waiting", "id": "abc-123", "livekit": null}"#;
        let resp: RequestEntryResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, LobbyStatus::Waiting);
        assert_eq!(resp.id, Some("abc-123".to_string()));
        assert!(resp.livekit.is_none());
    }

    #[test]
    fn parse_lobby_status_accepted_with_livekit() {
        let json = r#"{
            "status": "accepted",
            "id": "abc-123",
            "livekit": {
                "url": "https://livekit.example.com",
                "token": "eyJhbGciOiJ..."
            }
        }"#;
        let resp: RequestEntryResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, LobbyStatus::Accepted);
        assert!(resp.livekit.is_some());
        let lk = resp.livekit.unwrap();
        assert_eq!(lk.url, "https://livekit.example.com");
    }

    #[test]
    fn parse_lobby_status_denied() {
        let json = r#"{"status": "denied", "id": "abc-123", "livekit": null}"#;
        let resp: RequestEntryResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, LobbyStatus::Denied);
    }

    #[test]
    fn parse_lobby_status_unknown() {
        let json = r#"{"status": "expired", "id": "abc-123", "livekit": null}"#;
        let resp: RequestEntryResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, LobbyStatus::Unknown);
    }

    #[test]
    fn parse_waiting_participant() {
        let json = r#"{"id": "p-123", "username": "Alice"}"#;
        let wp: WaitingParticipant = serde_json::from_str(json).unwrap();
        assert_eq!(wp.id, "p-123");
        assert_eq!(wp.username, "Alice");
    }

    #[test]
    fn parse_waiting_participants_list() {
        let json = r#"[
            {"id": "p-1", "username": "Alice"},
            {"id": "p-2", "username": "Bob"}
        ]"#;
        let list: Vec<WaitingParticipant> = serde_json::from_str(json).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].username, "Alice");
        assert_eq!(list[1].username, "Bob");
    }

    #[tokio::test]
    async fn request_entry_with_invalid_url_returns_error() {
        let result = LobbyService::request_entry("invalid-url", "Alice").await;
        assert!(result.is_err());
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p visio-core`
Expected: All tests pass (existing + new)

**Step 3: Commit**

```bash
git add crates/visio-core/src/lobby.rs
git commit -m "test(core): add unit tests for LobbyService"
```

---

### Task 12: End-to-end manual testing

Test the complete flow across all platforms.

**Test scenario 1 — Participant waiting screen:**
1. Create a trusted room from the app (logged in)
2. In a different browser/device (not logged in), join the room URL
3. Verify the waiting screen appears with spinner and "Waiting for host approval..."
4. Verify the cancel button works and returns to home

**Test scenario 2 — Host accept flow:**
1. Create a trusted room, stay in the call
2. From another device, join as anonymous
3. Verify the host sees a notification/badge
4. Open participants panel, verify "Waiting room" section shows
5. Click "Admit" — verify the participant joins the call
6. Verify both sides see each other with audio/video

**Test scenario 3 — Host deny flow:**
1. Same setup as scenario 2
2. Click "Deny" instead
3. Verify the participant sees "Entry denied" and returns to home

**Test scenario 4 — Authenticated bypass:**
1. Create a trusted room
2. Log in on another device with the same Meet instance
3. Join the trusted room — should bypass lobby entirely

**Test on all 3 platforms:**
- Desktop: `cd crates/visio-desktop && cargo tauri dev`
- Android: Build and deploy APK
- iOS: Build and deploy to simulator/device
