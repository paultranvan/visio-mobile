# OIDC Authentication — Phase 1 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add OIDC authentication via platform-native secure browser APIs, with session cookie persistence and authenticated state on the home screen.

**Architecture:** New `session.rs` module in visio-core handles session state and cookie injection. Native layers (Android/iOS/Desktop) handle browser-based OIDC flow and secure cookie storage. The anonymous flow is unchanged.

**Tech Stack:** Rust (reqwest, serde), UniFFI, Kotlin (Custom Tabs, EncryptedSharedPreferences), Swift (ASWebAuthenticationSession, Keychain), Tauri (deep-link plugin, shell plugin)

**Design doc:** `docs/plans/2026-03-07-oidc-authentication-design.md`

---

### Task 1: Add `session.rs` module to visio-core

**Files:**
- Create: `crates/visio-core/src/session.rs`
- Modify: `crates/visio-core/src/lib.rs:6-18` (add module + export)
- Modify: `crates/visio-core/src/errors.rs:4-15` (add Session variant)

**Step 1: Write the failing test**

Add to `crates/visio-core/src/session.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_default_is_anonymous() {
        let session = SessionManager::new();
        assert!(matches!(session.state(), SessionState::Anonymous));
    }

    #[test]
    fn test_set_cookie_changes_state() {
        let mut session = SessionManager::new();
        let user = UserInfo {
            id: "123".to_string(),
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
        };
        session.set_authenticated(user.clone(), "abc123".to_string());
        match session.state() {
            SessionState::Authenticated { user: u, .. } => {
                assert_eq!(u.display_name, "Test User");
            }
            _ => panic!("Expected Authenticated state"),
        }
    }

    #[test]
    fn test_clear_session_returns_to_anonymous() {
        let mut session = SessionManager::new();
        let user = UserInfo {
            id: "123".to_string(),
            email: "test@example.com".to_string(),
            display_name: "Test".to_string(),
        };
        session.set_authenticated(user, "abc123".to_string());
        session.clear();
        assert!(matches!(session.state(), SessionState::Anonymous));
    }

    #[test]
    fn test_cookie_returns_none_when_anonymous() {
        let session = SessionManager::new();
        assert!(session.cookie().is_none());
    }

    #[test]
    fn test_cookie_returns_value_when_authenticated() {
        let mut session = SessionManager::new();
        let user = UserInfo {
            id: "1".to_string(),
            email: "a@b.com".to_string(),
            display_name: "A".to_string(),
        };
        session.set_authenticated(user, "mycookie".to_string());
        assert_eq!(session.cookie(), Some("mycookie".to_string()));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p visio-core --lib session`
Expected: FAIL — `session` module not found

**Step 3: Write minimal implementation**

Create `crates/visio-core/src/session.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub enum SessionState {
    Anonymous,
    Authenticated { user: UserInfo, cookie: String },
}

pub struct SessionManager {
    state: SessionState,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            state: SessionState::Anonymous,
        }
    }

    pub fn state(&self) -> &SessionState {
        &self.state
    }

    pub fn set_authenticated(&mut self, user: UserInfo, cookie: String) {
        self.state = SessionState::Authenticated { user, cookie };
    }

    pub fn clear(&mut self) {
        self.state = SessionState::Anonymous;
    }

    pub fn cookie(&self) -> Option<String> {
        match &self.state {
            SessionState::Authenticated { cookie, .. } => Some(cookie.clone()),
            SessionState::Anonymous => None,
        }
    }

    pub fn user(&self) -> Option<&UserInfo> {
        match &self.state {
            SessionState::Authenticated { user, .. } => Some(user),
            SessionState::Anonymous => None,
        }
    }
}
```

Add to `crates/visio-core/src/lib.rs` after line 14:

```rust
pub mod session;
```

Add to exports after line 18:

```rust
pub use session::{SessionManager, SessionState, UserInfo};
```

Add `Session(String)` variant to `VisioError` in `crates/visio-core/src/errors.rs`:

```rust
#[error("Session error: {0}")]
Session(String),
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p visio-core --lib session`
Expected: All 5 tests PASS

**Step 5: Commit**

```bash
git add crates/visio-core/src/session.rs crates/visio-core/src/lib.rs crates/visio-core/src/errors.rs
git commit -m "feat(core): add session module with SessionManager and UserInfo"
```

---

### Task 2: Add HTTP methods to SessionManager (validate_session, fetch_user, logout)

**Files:**
- Modify: `crates/visio-core/src/session.rs`

**Step 1: Write the failing tests**

Add to the `tests` module in `session.rs`:

```rust
    #[tokio::test]
    async fn test_fetch_user_with_invalid_cookie_returns_error() {
        // This test requires a running meet instance, skip in unit tests
        // It validates the API call structure
        let session = SessionManager::new();
        let result = SessionManager::fetch_user("https://meet.example.com", "invalid_cookie").await;
        // Network error expected (no server)
        assert!(result.is_err());
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p visio-core --lib session::tests::test_fetch_user`
Expected: FAIL — `fetch_user` method not found

**Step 3: Write implementation**

Add async methods to `SessionManager` in `session.rs`:

```rust
use crate::errors::VisioError;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue};

impl SessionManager {
    /// Call GET /api/v1.0/users/me/ with session cookie, return UserInfo
    pub async fn fetch_user(meet_url: &str, cookie: &str) -> Result<UserInfo, VisioError> {
        let url = format!("{}/api/v1.0/users/me/", meet_url.trim_end_matches('/'));

        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&format!("sessionid={}", cookie))
                .map_err(|e| VisioError::Session(e.to_string()))?,
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| VisioError::Http(e.to_string()))?;

        if response.status() == 401 || response.status() == 403 {
            return Err(VisioError::Session("Session expired or invalid".to_string()));
        }

        let user: UserInfo = response
            .json()
            .await
            .map_err(|e| VisioError::Session(format!("Failed to parse user info: {}", e)))?;

        Ok(user)
    }

    /// Validate current session by calling /users/me/
    pub async fn validate_session(&mut self, meet_url: &str) -> Result<bool, VisioError> {
        let cookie = match self.cookie() {
            Some(c) => c,
            None => return Ok(false),
        };

        match Self::fetch_user(meet_url, &cookie).await {
            Ok(user) => {
                self.state = SessionState::Authenticated {
                    user,
                    cookie,
                };
                Ok(true)
            }
            Err(_) => {
                self.clear();
                Ok(false)
            }
        }
    }

    /// Logout: call /logout on backend, then clear local state
    pub async fn logout(&mut self, meet_url: &str) -> Result<(), VisioError> {
        if let Some(cookie) = self.cookie() {
            let url = format!("{}/logout", meet_url.trim_end_matches('/'));

            let mut headers = HeaderMap::new();
            headers.insert(
                COOKIE,
                HeaderValue::from_str(&format!("sessionid={}", cookie))
                    .map_err(|e| VisioError::Session(e.to_string()))?,
            );

            let client = reqwest::Client::new();
            // Best-effort logout, ignore errors
            let _ = client.get(&url).headers(headers).send().await;
        }

        self.clear();
        Ok(())
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p visio-core --lib session`
Expected: All tests PASS (fetch_user test will get a network error as expected)

**Step 5: Commit**

```bash
git add crates/visio-core/src/session.rs
git commit -m "feat(core): add fetch_user, validate_session, and logout to SessionManager"
```

---

### Task 3: Inject session cookie into existing auth.rs HTTP requests

**Files:**
- Modify: `crates/visio-core/src/auth.rs:33-74` (request_token)

**Step 1: Write the failing test**

Add to `auth.rs` tests module:

```rust
    #[test]
    fn test_request_token_accepts_optional_cookie() {
        // Verify the function signature compiles with cookie parameter
        let _future = AuthService::request_token("https://example.com/room", Some("user"), Some("sessionid=abc"));
    }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p visio-core --lib auth::tests::test_request_token_accepts_optional_cookie`
Expected: FAIL — too many arguments

**Step 3: Modify `request_token` and `validate_room` to accept optional cookie**

In `crates/visio-core/src/auth.rs`, update `request_token` signature and body:

```rust
pub async fn request_token(
    meet_url: &str,
    username: Option<&str>,
    session_cookie: Option<&str>,
) -> Result<TokenInfo, VisioError> {
```

Add cookie header to the request (around line 47 where `reqwest::get()` is called):

```rust
let client = reqwest::Client::new();
let mut request = client.get(&api_url);

if let Some(cookie) = session_cookie {
    request = request.header(reqwest::header::COOKIE, format!("sessionid={}", cookie));
}

let response = request
    .send()
    .await
    .map_err(|e| VisioError::Http(e.to_string()))?;
```

Update `validate_room` to pass through the cookie:

```rust
pub async fn validate_room(
    meet_url: &str,
    username: Option<&str>,
    session_cookie: Option<&str>,
) -> Result<TokenInfo, VisioError> {
```

Update all existing callers to pass `None` for cookie where not applicable.

**Step 4: Fix all callers**

Search for all calls to `request_token` and `validate_room` in:
- `crates/visio-core/src/room.rs` — add `None` as cookie parameter
- `crates/visio-ffi/src/lib.rs:667` — will be updated in Task 4
- Existing tests in `auth.rs` — add `None` parameter

Run: `cargo test -p visio-core --lib`
Expected: All existing tests PASS + new test PASSES

**Step 5: Commit**

```bash
git add crates/visio-core/src/auth.rs crates/visio-core/src/room.rs
git commit -m "feat(core): accept optional session cookie in auth requests"
```

---

### Task 4: Expose session management via UniFFI

**Files:**
- Modify: `crates/visio-ffi/src/lib.rs:375-381` (VisioClient struct), `257-263` (types)
- Modify: `crates/visio-ffi/src/visio.udl:107-180` (interface)

**Step 1: Add SessionManager to VisioClient struct**

In `crates/visio-ffi/src/lib.rs`, add to the `VisioClient` struct:

```rust
session_manager: Arc<Mutex<visio_core::SessionManager>>,
```

Initialize in `VisioClient::new()`:

```rust
session_manager: Arc::new(Mutex::new(visio_core::SessionManager::new())),
```

**Step 2: Add FFI types for session**

Add to `crates/visio-ffi/src/lib.rs` types section:

```rust
#[derive(uniffi::Enum)]
pub enum SessionState {
    Anonymous,
    Authenticated { display_name: String, email: String },
}

#[derive(uniffi::Record)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub display_name: String,
}
```

**Step 3: Add methods to VisioClient**

```rust
/// Set session cookie after OIDC flow, validate with backend
pub async fn authenticate(&self, meet_url: String, cookie: String) -> Result<UserInfo, VisioError> {
    let user = visio_core::SessionManager::fetch_user(&meet_url, &cookie)
        .await
        .map_err(VisioError::from)?;

    let ffi_user = UserInfo {
        id: user.id.clone(),
        email: user.email.clone(),
        display_name: user.display_name.clone(),
    };

    let mut session = self.session_manager.lock().unwrap();
    session.set_authenticated(user, cookie);

    Ok(ffi_user)
}

/// Get current session state
pub fn get_session_state(&self) -> SessionState {
    let session = self.session_manager.lock().unwrap();
    match session.state() {
        visio_core::SessionState::Anonymous => SessionState::Anonymous,
        visio_core::SessionState::Authenticated { user, .. } => SessionState::Authenticated {
            display_name: user.display_name.clone(),
            email: user.email.clone(),
        },
    }
}

/// Logout and clear session
pub async fn logout(&self, meet_url: String) -> Result<(), VisioError> {
    let mut session = self.session_manager.lock().unwrap();
    session.logout(&meet_url).await.map_err(VisioError::from)?;
    Ok(())
}

/// Validate existing session cookie
pub async fn validate_session(&self, meet_url: String) -> Result<bool, VisioError> {
    let mut session = self.session_manager.lock().unwrap();
    session.validate_session(&meet_url).await.map_err(VisioError::from)?;
    match session.state() {
        visio_core::SessionState::Authenticated { .. } => Ok(true),
        _ => Ok(false),
    }
}
```

**Step 4: Update the `validate_room` FFI method to pass session cookie**

In the existing `validate_room` method (~line 663), inject the cookie:

```rust
pub async fn validate_room(&self, url: String, username: Option<String>) -> RoomValidationResult {
    let cookie = {
        let session = self.session_manager.lock().unwrap();
        session.cookie()
    };
    match visio_core::AuthService::validate_room(
        &url,
        username.as_deref(),
        cookie.as_deref(),
    ).await {
        // ... existing match arms
    }
}
```

**Step 5: Update UDL interface**

Add to `crates/visio-ffi/src/visio.udl`:

```
[Enum]
interface SessionState {
    Anonymous();
    Authenticated(string display_name, string email);
};

dictionary UserInfo {
    string id;
    string email;
    string display_name;
};
```

Add to the `VisioClient` interface block:

```
[Throws=VisioError]
UserInfo authenticate(string meet_url, string cookie);

SessionState get_session_state();

[Throws=VisioError]
void logout(string meet_url);

[Throws=VisioError]
boolean validate_session(string meet_url);
```

**Step 6: Add Session variant to FFI VisioError**

In the FFI `VisioError` enum and the `From<visio_core::VisioError>` impl, add the `Session` variant.

**Step 7: Regenerate bindings and verify build**

Run: `scripts/generate-bindings.sh all`
Run: `cargo build -p visio-ffi`
Expected: Build succeeds

**Step 8: Commit**

```bash
git add crates/visio-ffi/src/lib.rs crates/visio-ffi/src/visio.udl
git commit -m "feat(ffi): expose session management via UniFFI (authenticate, logout, validate_session)"
```

---

### Task 5: Add i18n keys for authentication (all 6 languages)

**Files:**
- Modify: `i18n/en.json`, `i18n/fr.json`, `i18n/de.json`, `i18n/es.json`, `i18n/it.json`, `i18n/nl.json`

**Step 1: Add keys to all 6 JSON files**

English (`en.json`):
```json
"home.connect": "Connect",
"home.logout": "Logout",
"home.loggedAs": "Logged in as"
```

French (`fr.json`):
```json
"home.connect": "Se connecter",
"home.logout": "Déconnexion",
"home.loggedAs": "Connecté en tant que"
```

German (`de.json`):
```json
"home.connect": "Anmelden",
"home.logout": "Abmelden",
"home.loggedAs": "Angemeldet als"
```

Spanish (`es.json`):
```json
"home.connect": "Conectarse",
"home.logout": "Cerrar sesión",
"home.loggedAs": "Conectado como"
```

Italian (`it.json`):
```json
"home.connect": "Accedi",
"home.logout": "Disconnetti",
"home.loggedAs": "Connesso come"
```

Dutch (`nl.json`):
```json
"home.connect": "Inloggen",
"home.logout": "Uitloggen",
"home.loggedAs": "Ingelogd als"
```

**Step 2: Verify JSON files are valid**

Run: `for f in i18n/*.json; do python3 -c "import json; json.load(open('$f'))"; echo "$f OK"; done`
Expected: All 6 files OK

**Step 3: Commit**

```bash
git add i18n/
git commit -m "feat(i18n): add authentication strings in all 6 languages"
```

---

### Task 6: Android — OIDC flow with Custom Tabs

**Files:**
- Modify: `android/app/build.gradle.kts:59-77` (add Custom Tabs + security-crypto dependencies)
- Create: `android/app/src/main/kotlin/io/visio/mobile/auth/OidcAuthManager.kt`
- Modify: `android/app/src/main/kotlin/io/visio/mobile/VisioManager.kt:46-78` (add session state)
- Modify: `android/app/src/main/kotlin/io/visio/mobile/MainActivity.kt:27-40` (handle auth callback)

**Step 1: Add dependencies to build.gradle.kts**

Add to the dependencies block:

```kotlin
implementation("androidx.browser:browser:1.8.0")
implementation("androidx.security:security-crypto:1.1.0-alpha06")
```

**Step 2: Create OidcAuthManager**

Create `android/app/src/main/kotlin/io/visio/mobile/auth/OidcAuthManager.kt`:

```kotlin
package io.visio.mobile.auth

import android.content.Context
import android.net.Uri
import androidx.browser.customtabs.CustomTabsIntent
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

class OidcAuthManager(context: Context) {
    private val masterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()

    private val prefs = EncryptedSharedPreferences.create(
        context,
        "visio_auth",
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )

    fun launchOidcFlow(context: android.app.Activity, meetInstance: String) {
        val authUrl = "https://$meetInstance/authenticate/?returnTo=https://$meetInstance/"
        val intent = CustomTabsIntent.Builder().build()
        intent.launchUrl(context, Uri.parse(authUrl))
    }

    fun saveCookie(cookie: String) {
        prefs.edit().putString("sessionid", cookie).apply()
    }

    fun getSavedCookie(): String? {
        return prefs.getString("sessionid", null)
    }

    fun clearCookie() {
        prefs.edit().remove("sessionid").apply()
    }
}
```

**Step 3: Add session state to VisioManager**

Add to `VisioManager.kt`:

```kotlin
var sessionState by mutableStateOf<SessionState>(SessionState.Anonymous)
    private set
var authenticatedUser by mutableStateOf<UserInfo?>(null)
    private set

lateinit var authManager: OidcAuthManager
    private set

fun initAuth(context: Context) {
    authManager = OidcAuthManager(context)
    // Try to restore session on launch
    val savedCookie = authManager.getSavedCookie()
    if (savedCookie != null) {
        CoroutineScope(Dispatchers.IO).launch {
            try {
                val meetInstance = client.getMeetInstances().firstOrNull() ?: return@launch
                val user = client.authenticate("https://$meetInstance", savedCookie)
                withContext(Dispatchers.Main) {
                    authenticatedUser = user
                    sessionState = SessionState.Authenticated(user.displayName, user.email)
                }
            } catch (e: Exception) {
                authManager.clearCookie()
            }
        }
    }
}

fun onAuthCookieReceived(cookie: String) {
    authManager.saveCookie(cookie)
    CoroutineScope(Dispatchers.IO).launch {
        try {
            val meetInstance = client.getMeetInstances().firstOrNull() ?: return@launch
            val user = client.authenticate("https://$meetInstance", cookie)
            withContext(Dispatchers.Main) {
                authenticatedUser = user
                sessionState = SessionState.Authenticated(user.displayName, user.email)
                displayName = user.displayName
            }
        } catch (e: Exception) {
            authManager.clearCookie()
        }
    }
}

fun logout() {
    CoroutineScope(Dispatchers.IO).launch {
        try {
            val meetInstance = client.getMeetInstances().firstOrNull() ?: return@launch
            client.logout("https://$meetInstance")
        } catch (_: Exception) {}
        authManager.clearCookie()
        withContext(Dispatchers.Main) {
            authenticatedUser = null
            sessionState = SessionState.Anonymous
        }
    }
}
```

**Step 4: Initialize auth in MainActivity.onCreate**

In `MainActivity.kt`, add after `VisioManager.initialize(this)`:

```kotlin
VisioManager.initAuth(this)
```

**Step 5: Build and verify**

Run: `cd android && ./gradlew assembleDebug`
Expected: Build succeeds

**Step 6: Commit**

```bash
git add android/
git commit -m "feat(android): add OIDC auth manager with Custom Tabs and EncryptedSharedPreferences"
```

---

### Task 7: Android — Update HomeScreen UI with Connect/Logout

**Files:**
- Modify: `android/app/src/main/kotlin/io/visio/mobile/ui/HomeScreen.kt:128-295`

**Step 1: Add Connect/Logout section to HomeScreen**

Insert between the logo/title section and the room URL input (~line 180, after the subtitle):

```kotlin
// Authentication section
when (VisioManager.sessionState) {
    is SessionState.Anonymous -> {
        Button(
            onClick = {
                val meetInstance = meetInstances.firstOrNull() ?: return@Button
                VisioManager.authManager.launchOidcFlow(context as android.app.Activity, meetInstance)
            },
            modifier = Modifier.fillMaxWidth(),
            colors = ButtonDefaults.outlinedButtonColors()
        ) {
            Text(Strings.t("home.connect", lang))
        }
    }
    is SessionState.Authenticated -> {
        val user = VisioManager.authenticatedUser
        if (user != null) {
            Text(
                text = "${Strings.t("home.loggedAs", lang)} ${user.displayName}",
                style = MaterialTheme.typography.bodyMedium,
                color = VisioColors.textSecondary
            )
            TextButton(onClick = { VisioManager.logout() }) {
                Text(Strings.t("home.logout", lang))
            }
        }
    }
}
```

**Step 2: Pre-fill display name from OIDC identity**

In the `LaunchedEffect` that initializes state, add:

```kotlin
if (VisioManager.authenticatedUser != null && username.isEmpty()) {
    username = VisioManager.authenticatedUser!!.displayName
}
```

**Step 3: Build and verify**

Run: `cd android && ./gradlew assembleDebug`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add android/app/src/main/kotlin/io/visio/mobile/ui/HomeScreen.kt
git commit -m "feat(android): add Connect/Logout UI on home screen"
```

---

### Task 8: iOS — OIDC flow with ASWebAuthenticationSession

**Files:**
- Create: `ios/VisioMobile/Auth/OidcAuthManager.swift`
- Modify: `ios/VisioMobile/VisioManager.swift:15-30` (add session state)

**Step 1: Create OidcAuthManager**

Create `ios/VisioMobile/Auth/OidcAuthManager.swift`:

```swift
import AuthenticationServices
import Security

class OidcAuthManager: NSObject, ASWebAuthenticationPresentationContextProviding {

    func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        guard let scene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
              let window = scene.windows.first else {
            return ASPresentationAnchor()
        }
        return window
    }

    func launchOidcFlow(meetInstance: String, completion: @escaping (String?) -> Void) {
        let authURL = URL(string: "https://\(meetInstance)/authenticate/?returnTo=https://\(meetInstance)/")!
        let callbackScheme = "visio"

        let session = ASWebAuthenticationSession(url: authURL, callbackURLScheme: callbackScheme) { callbackURL, error in
            guard error == nil, let url = callbackURL else {
                completion(nil)
                return
            }
            // Extract session cookie from the callback
            // ASWebAuthenticationSession shares cookies with Safari
            // We need to extract the sessionid from the shared cookie store
            let cookies = HTTPCookieStorage.shared.cookies(for: URL(string: "https://\(meetInstance)")!) ?? []
            let sessionCookie = cookies.first(where: { $0.name == "sessionid" })?.value
            completion(sessionCookie)
        }

        session.presentationContextProvider = self
        session.prefersEphemeralWebBrowserSession = false // Share cookies with Safari
        session.start()
    }

    // MARK: - Keychain Storage

    func saveCookie(_ cookie: String) {
        let data = cookie.data(using: .utf8)!
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: "visio_sessionid",
            kSecAttrService as String: "io.visio.mobile",
        ]
        SecItemDelete(query as CFDictionary)
        var addQuery = query
        addQuery[kSecValueData as String] = data
        SecItemAdd(addQuery as CFDictionary, nil)
    }

    func getSavedCookie() -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: "visio_sessionid",
            kSecAttrService as String: "io.visio.mobile",
            kSecReturnData as String: true,
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let data = result as? Data else { return nil }
        return String(data: data, encoding: .utf8)
    }

    func clearCookie() {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: "visio_sessionid",
            kSecAttrService as String: "io.visio.mobile",
        ]
        SecItemDelete(query as CFDictionary)
    }
}
```

**Step 2: Add session state to VisioManager**

Add to `ios/VisioMobile/VisioManager.swift`:

```swift
@Published var sessionState: SessionState = .anonymous
@Published var authenticatedUser: UserInfo? = nil
let authManager = OidcAuthManager()

func initAuth() {
    guard let cookie = authManager.getSavedCookie() else { return }
    guard let meetInstance = client.getMeetInstances().first else { return }

    DispatchQueue.global().async { [weak self] in
        do {
            let user = try self?.client.authenticate(meetUrl: "https://\(meetInstance)", cookie: cookie)
            DispatchQueue.main.async {
                self?.authenticatedUser = user
                self?.sessionState = .authenticated(displayName: user?.displayName ?? "", email: user?.email ?? "")
                if let name = user?.displayName {
                    self?.displayName = name
                }
            }
        } catch {
            self?.authManager.clearCookie()
        }
    }
}

func onAuthCookieReceived(_ cookie: String) {
    authManager.saveCookie(cookie)
    guard let meetInstance = client.getMeetInstances().first else { return }

    DispatchQueue.global().async { [weak self] in
        do {
            let user = try self?.client.authenticate(meetUrl: "https://\(meetInstance)", cookie: cookie)
            DispatchQueue.main.async {
                self?.authenticatedUser = user
                self?.sessionState = .authenticated(displayName: user?.displayName ?? "", email: user?.email ?? "")
                if let name = user?.displayName {
                    self?.displayName = name
                }
            }
        } catch {
            self?.authManager.clearCookie()
        }
    }
}

func logout() {
    guard let meetInstance = client.getMeetInstances().first else { return }
    DispatchQueue.global().async { [weak self] in
        try? self?.client.logout(meetUrl: "https://\(meetInstance)")
        self?.authManager.clearCookie()
        DispatchQueue.main.async {
            self?.authenticatedUser = nil
            self?.sessionState = .anonymous
        }
    }
}
```

**Step 3: Call initAuth on app launch**

In `VisioMobileApp.swift`, add after the VisioManager initialization:

```swift
.onAppear {
    manager.initAuth()
}
```

**Step 4: Build and verify**

Run: `scripts/build-ios.sh sim`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add ios/VisioMobile/Auth/OidcAuthManager.swift ios/VisioMobile/VisioManager.swift ios/VisioMobile/VisioMobileApp.swift
git commit -m "feat(ios): add OIDC auth manager with ASWebAuthenticationSession and Keychain"
```

---

### Task 9: iOS — Update HomeView UI with Connect/Logout

**Files:**
- Modify: `ios/VisioMobile/Views/HomeView.swift:36-104`

**Step 1: Add Connect/Logout section to HomeView**

Insert between the subtitle and room URL input:

```swift
// Authentication section
switch manager.sessionState {
case .anonymous:
    Button(action: {
        guard let meetInstance = meetInstances.first else { return }
        manager.authManager.launchOidcFlow(meetInstance: meetInstance) { cookie in
            if let cookie = cookie {
                manager.onAuthCookieReceived(cookie)
            }
        }
    }) {
        Label(Strings.t("home.connect", lang: lang), systemImage: "person.circle")
            .frame(maxWidth: .infinity)
    }
    .buttonStyle(.bordered)

case .authenticated(let displayName, _):
    VStack(spacing: 4) {
        Text("\(Strings.t("home.loggedAs", lang: lang)) \(displayName)")
            .font(.subheadline)
            .foregroundStyle(.secondary)
        Button(Strings.t("home.logout", lang: lang), action: {
            manager.logout()
        })
        .font(.subheadline)
    }
}
```

**Step 2: Pre-fill display name**

Add to the `.onAppear` or `.task` modifier:

```swift
if let user = manager.authenticatedUser, displayName.isEmpty {
    displayName = user.displayName
}
```

**Step 3: Build and verify**

Run: `scripts/build-ios.sh sim`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add ios/VisioMobile/Views/HomeView.swift
git commit -m "feat(ios): add Connect/Logout UI on home screen"
```

---

### Task 10: Desktop (Tauri) — OIDC flow with system browser

**Files:**
- Modify: `crates/visio-desktop/src/lib.rs` (add Tauri commands for auth)
- Modify: `crates/visio-desktop/frontend/src/App.tsx` (add Connect/Logout UI)
- Modify: `crates/visio-desktop/Cargo.toml` (ensure deep-link plugin is configured)

**Step 1: Add Tauri auth commands**

In `crates/visio-desktop/src/lib.rs`, add commands:

```rust
#[tauri::command]
async fn launch_oidc(meet_instance: String) -> Result<(), String> {
    let url = format!("https://{}/authenticate/?returnTo=visio://auth-callback", meet_instance);
    tauri::api::shell::open(&url, None).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn authenticate(meet_url: String, cookie: String, state: tauri::State<'_, AppState>) -> Result<UserInfo, String> {
    let user = visio_core::SessionManager::fetch_user(&meet_url, &cookie)
        .await
        .map_err(|e| e.to_string())?;
    // Store in app state
    let mut session = state.session.lock().unwrap();
    session.set_authenticated(user.clone(), cookie);
    Ok(user.into())
}

#[tauri::command]
async fn logout(meet_url: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut session = state.session.lock().unwrap();
    session.logout(&meet_url).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_session_state(state: tauri::State<'_, AppState>) -> SessionState {
    let session = state.session.lock().unwrap();
    session.state().clone().into()
}
```

**Step 2: Handle deep link callback**

Register the `visio://auth-callback` deep link handler in the Tauri setup to extract the cookie and call `authenticate`.

**Step 3: Update frontend**

Add Connect/Logout buttons in `App.tsx` following the same pattern as mobile (invoke Tauri commands).

**Step 4: Build and verify**

Run: `cd crates/visio-desktop && cargo tauri build --debug`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add crates/visio-desktop/
git commit -m "feat(desktop): add OIDC auth flow with system browser and deep link callback"
```

---

### Task 11: End-to-end manual testing

**Prerequisites:** A running Meet instance with OIDC configured.

**Test matrix:**

| Test Case | Android | iOS | Desktop |
|-----------|---------|-----|---------|
| Connect button visible when anonymous | | | |
| OIDC flow opens browser | | | |
| After auth, user name displayed | | | |
| Display name pre-filled | | | |
| Logout returns to anonymous | | | |
| Session persists after app restart | | | |
| Expired session handled gracefully | | | |
| Join room works while authenticated | | | |
| Join room still works when anonymous | | | |

**Step 1: Test on each platform**

Follow the test matrix above.

**Step 2: Fix any issues found**

**Step 3: Final commit**

```bash
git commit -m "fix: address issues found during OIDC auth testing"
```
