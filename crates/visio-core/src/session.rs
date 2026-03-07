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

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
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
