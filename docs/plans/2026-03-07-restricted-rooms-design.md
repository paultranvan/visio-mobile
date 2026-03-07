# Restricted Rooms with Invitation Management — Design

## Scope

Add `restricted` access level to room creation, with full invitation management via user search autocomplete. Users can add/remove members at creation time and during the call. Across all 3 platforms (Desktop, Android, iOS).

## Meet API Endpoints

| Endpoint | Method | Role |
|----------|--------|------|
| `GET /api/v1.0/users/?q={email}` | GET | Search users by email (trigram similarity, requires `ALLOW_UNSECURE_USER_LISTING`) |
| `GET /api/v1.0/resource-accesses/?resource={room_id}` | GET | List accesses for a room (requires admin/owner role) |
| `POST /api/v1.0/resource-accesses/` | POST | Add access: `{"user": "<uuid>", "resource": "<room_id>", "role": "member"}` |
| `DELETE /api/v1.0/resource-accesses/{id}/` | DELETE | Revoke access (requires admin/owner role) |

## Access Level Behavior

From Meet backend `lobby_service.py`:

- **public**: anyone joins directly, no lobby
- **trusted**: authenticated users bypass lobby, anonymous go through lobby (waiting room)
- **restricted**: everyone goes through lobby UNLESS they have an explicit `ResourceAccess` (owner/admin/member), in which case `role is not None` grants direct access via the room serializer

## Flow — Room Creation

1. Host selects `restricted` in access level picker (3rd option)
2. Autocomplete field appears: "Invite members"
3. Host types email → dropdown with matching users → selects → chip/tag added below
4. Can add multiple, remove with ✕ on each chip
5. Clicks "Create" → room created → `POST /resource-accesses/` for each invited user (role: `member`)
6. Host auto-joins the room

## Flow — In-Call (Room Info Tab)

For restricted rooms only, when host is authenticated:

- **Members section** showing current accesses (`GET /resource-accesses/?resource={room_id}`)
- Each member: name + role (owner/admin/member) + remove button (except self)
- Autocomplete field at top to add new members
- Remove calls `DELETE /resource-accesses/{access_id}/`

## Components to Modify

| Layer | Files | Change |
|-------|-------|--------|
| **visio-core** | `access.rs` (new) | `search_users()`, `list_accesses()`, `add_access()`, `remove_access()` |
| **visio-core** | `lib.rs` | Register module, add re-exports |
| **visio-ffi** | `lib.rs`, `visio.udl` | Expose 4 methods + `UserSearchResult`, `RoomAccess` types |
| **Desktop** | `lib.rs` | Tauri commands for search/list/add/remove |
| **Desktop** | `App.tsx`, `App.css` | Autocomplete in create dialog + members section in Room Info |
| **Android** | `HomeScreen.kt` | Autocomplete in create room dialog |
| **Android** | `InCallSettingsSheet.kt` | Members section in Room Info tab |
| **iOS** | `HomeView.swift` | Autocomplete in create room sheet |
| **iOS** | `InCallSettingsSheet.swift` | Members section in Room Info tab |
| **i18n** | `*.json` | New keys for restricted, invite, members, remove (6 languages) |

## Types

```
UserSearchResult { id: String, email: String, full_name: Option<String>, short_name: Option<String> }
RoomAccess { id: String, user: UserSearchResult, resource: String, role: String }
```

## Error Handling

- `ALLOW_UNSECURE_USER_LISTING` disabled → autocomplete returns empty → show "User search not available on this server"
- User already invited → API returns 400 (unique constraint) → show "Already invited" or ignore silently
- Not authenticated → accesses endpoints return 401/403 → disable invitation UI
- Network errors → standard error handling, retry-friendly

## UI — Autocomplete

- Debounced input (300ms)
- Minimum 3 characters before searching
- Dropdown with user results: display name + email
- Selected users shown as dismissible chips/tags below the input
- Same component reused in creation dialog and in-call settings
