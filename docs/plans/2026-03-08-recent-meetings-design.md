# Recent Meetings History — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show the last 3 meetings on the home screen so users can one-tap rejoin.

**Architecture:** Add `RecentMeeting` struct to the existing `Settings`/`SettingsStore` in visio-core. Record meetings on successful connect. Expose via UniFFI (Android/iOS) and Tauri commands (Desktop). Each platform renders a "Recent" list below the URL input.

**Tech Stack:** Rust (visio-core, visio-ffi, visio-desktop), Kotlin/Compose (Android), Swift/SwiftUI (iOS), React/TypeScript (Desktop)

---

### Task 1: Add RecentMeeting struct and SettingsStore methods (Rust core)

**Files:**
- Modify: `crates/visio-core/src/settings.rs`

**Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block at the bottom of `crates/visio-core/src/settings.rs`:

```rust
#[test]
fn test_default_recent_meetings_empty() {
    let s = Settings::default();
    assert!(s.recent_meetings.is_empty());
}

#[test]
fn test_add_recent_meeting() {
    let dir = temp_dir();
    let path = dir.path().to_str().unwrap();
    let store = SettingsStore::new(path);
    store.add_recent_meeting("abc-defg-hij".to_string(), "meet.example.com".to_string());
    let recent = store.get_recent_meetings();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].slug, "abc-defg-hij");
    assert_eq!(recent[0].server, "meet.example.com");
    assert!(recent[0].timestamp_ms > 0);
}

#[test]
fn test_recent_meetings_caps_at_three() {
    let dir = temp_dir();
    let path = dir.path().to_str().unwrap();
    let store = SettingsStore::new(path);
    store.add_recent_meeting("aaa-bbbb-ccc".to_string(), "s.com".to_string());
    store.add_recent_meeting("ddd-eeee-fff".to_string(), "s.com".to_string());
    store.add_recent_meeting("ggg-hhhh-iii".to_string(), "s.com".to_string());
    store.add_recent_meeting("jjj-kkkk-lll".to_string(), "s.com".to_string());
    let recent = store.get_recent_meetings();
    assert_eq!(recent.len(), 3);
    assert_eq!(recent[0].slug, "jjj-kkkk-lll"); // most recent first
}

#[test]
fn test_recent_meetings_deduplicates() {
    let dir = temp_dir();
    let path = dir.path().to_str().unwrap();
    let store = SettingsStore::new(path);
    store.add_recent_meeting("abc-defg-hij".to_string(), "s.com".to_string());
    store.add_recent_meeting("ddd-eeee-fff".to_string(), "s.com".to_string());
    store.add_recent_meeting("abc-defg-hij".to_string(), "s.com".to_string());
    let recent = store.get_recent_meetings();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].slug, "abc-defg-hij"); // moved to front
}

#[test]
fn test_recent_meetings_persists() {
    let dir = temp_dir();
    let path = dir.path().to_str().unwrap();
    {
        let store = SettingsStore::new(path);
        store.add_recent_meeting("abc-defg-hij".to_string(), "s.com".to_string());
    }
    let store = SettingsStore::new(path);
    let recent = store.get_recent_meetings();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].slug, "abc-defg-hij");
}

#[test]
fn test_partial_json_defaults_recent_meetings() {
    let dir = temp_dir();
    let path = dir.path().to_str().unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{"display_name":"Eve"}"#,
    )
    .unwrap();
    let store = SettingsStore::new(path);
    assert!(store.get_recent_meetings().is_empty());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p visio-core --lib`
Expected: FAIL — `RecentMeeting` struct and methods don't exist yet.

**Step 3: Write the implementation**

Add the `RecentMeeting` struct above the `Settings` struct:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RecentMeeting {
    pub slug: String,
    pub server: String,
    pub timestamp_ms: u64,
}
```

Add field to `Settings`:

```rust
#[serde(default)]
pub recent_meetings: Vec<RecentMeeting>,
```

Add to `Settings::default()`:

```rust
recent_meetings: Vec::new(),
```

Add methods to `SettingsStore`:

```rust
pub fn get_recent_meetings(&self) -> Vec<RecentMeeting> {
    self.settings.lock().unwrap_or_else(|e| e.into_inner()).recent_meetings.clone()
}

pub fn add_recent_meeting(&self, slug: String, server: String) {
    let mut settings = self.settings.lock().unwrap_or_else(|e| e.into_inner());
    // Remove existing entry with same slug+server
    settings.recent_meetings.retain(|m| !(m.slug == slug && m.server == server));
    // Push to front
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    settings.recent_meetings.insert(0, RecentMeeting { slug, server, timestamp_ms: now });
    // Cap at 3
    settings.recent_meetings.truncate(3);
    drop(settings);
    self.save();
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p visio-core --lib`
Expected: All tests PASS.

**Step 5: Commit**

```bash
git add crates/visio-core/src/settings.rs
git commit -m "feat: add RecentMeeting to SettingsStore"
```

---

### Task 2: Record meetings on successful connect (Rust core)

**Files:**
- Modify: `crates/visio-core/src/room.rs`
- Modify: `crates/visio-core/src/lib.rs` (if needed for re-exports)

**Step 1: Add `SettingsStore` to `RoomManager`**

In `crates/visio-core/src/room.rs`, add a field to `RoomManager`:

```rust
settings_store: Option<Arc<crate::SettingsStore>>,
```

Initialize it as `None` in `RoomManager::new()` and `Default`.

Add a setter method:

```rust
pub fn set_settings_store(&mut self, store: Arc<crate::SettingsStore>) {
    self.settings_store = Some(store);
}
```

**Step 2: Record meeting in `connect()` on success**

In `RoomManager::connect()`, after `self.connect_with_token(...)` succeeds and before `Ok(())`, add:

```rust
// Record in recent meetings history
if let Some(store) = &self.settings_store {
    if let Ok((server, slug)) = crate::AuthService::parse_meet_url(meet_url) {
        store.add_recent_meeting(slug, server);
    }
}
```

Note: `parse_meet_url` is currently private. Make it `pub(crate)` so `room.rs` can use it.

**Step 3: Make `parse_meet_url` pub(crate)**

In `crates/visio-core/src/auth.rs`, change:

```rust
fn parse_meet_url(url: &str) -> Result<(String, String), VisioError> {
```

to:

```rust
pub(crate) fn parse_meet_url(url: &str) -> Result<(String, String), VisioError> {
```

**Step 4: Run tests**

Run: `cargo test -p visio-core --lib`
Expected: All tests PASS (no integration tests needed — connect requires LiveKit server).

**Step 5: Commit**

```bash
git add crates/visio-core/src/room.rs crates/visio-core/src/auth.rs crates/visio-core/src/lib.rs
git commit -m "feat: record meeting history on connect"
```

---

### Task 3: Wire SettingsStore into FFI VisioClient (visio-ffi)

**Files:**
- Modify: `crates/visio-ffi/src/visio.udl`
- Modify: `crates/visio-ffi/src/lib.rs`

**Step 1: Add RecentMeeting dictionary and method to UDL**

In `crates/visio-ffi/src/visio.udl`, add the dictionary (after the existing `Settings` dictionary):

```
dictionary RecentMeeting {
    string slug;
    string server;
    u64 timestamp_ms;
};
```

Add method to `interface VisioClient`:

```
sequence<RecentMeeting> get_recent_meetings();
```

**Step 2: Add FFI type and conversion in lib.rs**

Add the FFI struct:

```rust
#[derive(Debug, Clone)]
pub struct RecentMeeting {
    pub slug: String,
    pub server: String,
    pub timestamp_ms: u64,
}

impl From<visio_core::RecentMeeting> for RecentMeeting {
    fn from(m: visio_core::RecentMeeting) -> Self {
        Self {
            slug: m.slug,
            server: m.server,
            timestamp_ms: m.timestamp_ms,
        }
    }
}
```

**Step 3: Pass SettingsStore to RoomManager on VisioClient construction**

In `VisioClient::new()`, wrap `settings` in `Arc` and pass to `room_manager`:

Change the `settings` field type in `VisioClient` from `visio_core::SettingsStore` to `Arc<visio_core::SettingsStore>`.

In `VisioClient::new()`:

```rust
let settings = Arc::new(visio_core::SettingsStore::new(&data_dir));
let mut room_manager = visio_core::RoomManager::new();
room_manager.set_settings_store(settings.clone());
```

Update all `self.settings.xxx()` calls to work with Arc (they already do since Arc auto-derefs).

**Step 4: Add the get_recent_meetings method to VisioClient**

```rust
pub fn get_recent_meetings(&self) -> Vec<RecentMeeting> {
    self.settings.get_recent_meetings().into_iter().map(RecentMeeting::from).collect()
}
```

**Step 5: Verify compilation**

Run: `cargo check -p visio-ffi`
Expected: Compiles without errors.

**Step 6: Commit**

```bash
git add crates/visio-ffi/src/visio.udl crates/visio-ffi/src/lib.rs
git commit -m "feat: expose RecentMeeting via UniFFI"
```

---

### Task 4: Add Tauri command for Desktop (visio-desktop)

**Files:**
- Modify: `crates/visio-desktop/src/lib.rs`

**Step 1: Add SettingsStore to VisioState as Arc**

Change `settings: SettingsStore` to `settings: Arc<SettingsStore>` in `struct VisioState`.

In the setup where `VisioState` is created, wrap settings in `Arc::new()` and pass to `room_manager.set_settings_store(settings.clone())`.

**Step 2: Add Tauri command**

```rust
#[tauri::command]
fn get_recent_meetings(state: tauri::State<'_, VisioState>) -> Vec<serde_json::Value> {
    state.settings.get_recent_meetings().into_iter().map(|m| {
        serde_json::json!({
            "slug": m.slug,
            "server": m.server,
            "timestamp_ms": m.timestamp_ms,
        })
    }).collect()
}
```

**Step 3: Register the command**

Find the `invoke_handler` call and add `get_recent_meetings` to the list.

**Step 4: Verify compilation**

Run: `cargo check -p visio-desktop`
Expected: Compiles without errors.

**Step 5: Commit**

```bash
git add crates/visio-desktop/src/lib.rs
git commit -m "feat: add get_recent_meetings Tauri command"
```

---

### Task 5: Add i18n key

**Files:**
- Modify: `i18n/en.json`, `i18n/fr.json`, `i18n/de.json`, `i18n/es.json`, `i18n/it.json`, `i18n/nl.json`

**Step 1: Add the key to all 6 files**

Add before the closing `}` in each file:

- `en.json`: `"home.recent": "Recent"`
- `fr.json`: `"home.recent": "Récents"`
- `de.json`: `"home.recent": "Zuletzt"`
- `es.json`: `"home.recent": "Recientes"`
- `it.json`: `"home.recent": "Recenti"`
- `nl.json`: `"home.recent": "Recent"`

**Step 2: Commit**

```bash
git add i18n/
git commit -m "feat(i18n): add home.recent key"
```

---

### Task 6: Android UI — Recent meetings list

**Files:**
- Modify: `android/app/src/main/kotlin/io/visio/mobile/ui/HomeScreen.kt`

**Step 1: Load recent meetings**

In `HomeScreen`, add state:

```kotlin
var recentMeetings by remember { mutableStateOf(listOf<uniffi.visio.RecentMeeting>()) }
```

In the existing `LaunchedEffect(Unit)` that loads meet instances, also load:

```kotlin
recentMeetings = VisioManager.client.getRecentMeetings()
```

**Step 2: Add the Recent section UI**

After the room status `when` block and before the `Spacer(modifier = Modifier.height(16.dp))` that precedes the username field, add:

```kotlin
if (recentMeetings.isNotEmpty()) {
    Spacer(modifier = Modifier.height(12.dp))
    Text(
        text = Strings.t("home.recent", lang),
        style = MaterialTheme.typography.labelSmall,
        color = if (isDark) VisioColors.Greyscale400 else VisioColors.LightTextSecondary,
        modifier = Modifier.fillMaxWidth(),
    )
    Spacer(modifier = Modifier.height(4.dp))
    recentMeetings.forEach { meeting ->
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clickable { roomUrl = meeting.slug }
                .padding(vertical = 8.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = meeting.slug,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onBackground,
            )
            Text(
                text = formatRelativeTime(meeting.timestampMs),
                style = MaterialTheme.typography.bodySmall,
                color = if (isDark) VisioColors.Greyscale400 else VisioColors.LightTextSecondary,
            )
        }
    }
}
```

**Step 3: Add relative time formatting helper**

Add a composable-level helper function in the same file:

```kotlin
@Composable
private fun formatRelativeTime(timestampMs: ULong): String {
    val now = System.currentTimeMillis()
    val diff = now - timestampMs.toLong()
    val minutes = diff / 60_000
    val hours = minutes / 60
    val days = hours / 24
    return when {
        minutes < 1 -> "just now"
        minutes < 60 -> "${minutes}m ago"
        hours < 24 -> "${hours}h ago"
        days < 7 -> "${days}d ago"
        else -> {
            val sdf = java.text.SimpleDateFormat("MMM d", java.util.Locale.getDefault())
            sdf.format(java.util.Date(timestampMs.toLong()))
        }
    }
}
```

**Step 4: Add missing imports**

Add to imports:
```kotlin
import androidx.compose.foundation.clickable
```

**Step 5: Verify build**

Run: `cd android && ./gradlew ktlintCheck`
Expected: PASS (may need formatting fixes).

**Step 6: Commit**

```bash
git add android/app/src/main/kotlin/io/visio/mobile/ui/HomeScreen.kt
git commit -m "feat(android): add recent meetings on home"
```

---

### Task 7: iOS UI — Recent meetings list

**Files:**
- Modify: `ios/VisioMobile/Views/HomeView.swift`

**Step 1: Load recent meetings**

Add state property:

```swift
@State private var recentMeetings: [RecentMeeting] = []
```

In the `.onAppear` block, add:

```swift
recentMeetings = manager.client.getRecentMeetings()
```

**Step 2: Add the Recent section UI**

After the room status `if/else if` block and before the `TextField` for display name, add:

```swift
if !recentMeetings.isEmpty {
    VStack(alignment: .leading, spacing: 4) {
        Text(Strings.t("home.recent", lang: lang))
            .font(.caption)
            .foregroundStyle(VisioColors.secondaryText(dark: isDark))

        ForEach(recentMeetings, id: \.slug) { meeting in
            Button {
                roomURL = meeting.slug
            } label: {
                HStack {
                    Text(meeting.slug)
                        .font(.body)
                        .foregroundStyle(VisioColors.onBackground(dark: isDark))
                    Spacer()
                    Text(formatRelativeTime(meeting.timestampMs))
                        .font(.caption)
                        .foregroundStyle(VisioColors.secondaryText(dark: isDark))
                }
                .padding(.vertical, 6)
            }
            .buttonStyle(.plain)
        }
    }
}
```

**Step 3: Add relative time formatting helper**

Add a private method to `HomeView`:

```swift
private func formatRelativeTime(_ timestampMs: UInt64) -> String {
    let date = Date(timeIntervalSince1970: Double(timestampMs) / 1000.0)
    let formatter = RelativeDateTimeFormatter()
    formatter.unitsStyle = .abbreviated
    return formatter.localizedString(for: date, relativeTo: Date())
}
```

**Step 4: Commit**

```bash
git add ios/VisioMobile/Views/HomeView.swift
git commit -m "feat(ios): add recent meetings on home"
```

---

### Task 8: Desktop UI — Recent meetings list

**Files:**
- Modify: `crates/visio-desktop/frontend/src/App.tsx`

**Step 1: Load recent meetings in HomeSection**

Add state and load in `HomeSection`:

```typescript
const [recentMeetings, setRecentMeetings] = useState<{slug: string, server: string, timestamp_ms: number}[]>([]);

useEffect(() => {
  invoke<{slug: string, server: string, timestamp_ms: number}[]>("get_recent_meetings")
    .then(setRecentMeetings)
    .catch(() => {});
}, []);
```

**Step 2: Add the Recent section UI**

After the room status divs and before the username `form-group`, add:

```tsx
{recentMeetings.length > 0 && (
  <div className="recent-meetings">
    <label>{t("home.recent")}</label>
    {recentMeetings.map((m) => (
      <button
        key={m.slug + m.server}
        className="recent-meeting-row"
        onClick={() => setMeetUrl(m.slug)}
      >
        <span className="recent-slug">{m.slug}</span>
        <span className="recent-time">{formatRelativeTime(m.timestamp_ms)}</span>
      </button>
    ))}
  </div>
)}
```

**Step 3: Add relative time formatting helper**

Add a function at the top of the file or inside the component:

```typescript
function formatRelativeTime(timestampMs: number): string {
  const diff = Date.now() - timestampMs;
  const minutes = Math.floor(diff / 60_000);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  if (days < 7) return `${days}d ago`;
  return new Date(timestampMs).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}
```

**Step 4: Add CSS styles**

In `crates/visio-desktop/frontend/src/App.css`, add:

```css
.recent-meetings {
  margin-top: 4px;
  width: 100%;
}

.recent-meetings label {
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-bottom: 4px;
  display: block;
}

.recent-meeting-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
  padding: 6px 8px;
  border: none;
  background: transparent;
  border-radius: 6px;
  cursor: pointer;
  color: var(--text-primary);
  font-size: 0.9rem;
}

.recent-meeting-row:hover {
  background: var(--surface-hover, rgba(128, 128, 128, 0.1));
}

.recent-time {
  font-size: 0.75rem;
  color: var(--text-secondary);
}
```

**Step 5: Verify build**

Run: `cd crates/visio-desktop/frontend && npm run build`
Expected: Compiles without errors.

**Step 6: Commit**

```bash
git add crates/visio-desktop/frontend/src/App.tsx crates/visio-desktop/frontend/src/App.css
git commit -m "feat(desktop): add recent meetings on home"
```

---

### Task 9: Final verification

**Step 1: Run Rust tests**

Run: `cargo test -p visio-core --lib`
Expected: All tests PASS.

**Step 2: Run format and lint checks**

Run: `cargo fmt --check && cargo clippy -p visio-core -p visio-ffi`
Expected: PASS.

**Step 3: Commit any fixes if needed**
