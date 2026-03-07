use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RecentMeeting {
    pub slug: String,
    pub server: String,
    pub timestamp_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Settings {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default = "default_true")]
    pub mic_enabled_on_join: bool,
    #[serde(default)]
    pub camera_enabled_on_join: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_meet_instances")]
    pub meet_instances: Vec<String>,
    #[serde(default = "default_true")]
    pub notification_participant_join: bool,
    #[serde(default = "default_true")]
    pub notification_hand_raised: bool,
    #[serde(default = "default_true")]
    pub notification_message_received: bool,
    #[serde(default = "default_background_mode")]
    pub background_mode: String,
    #[serde(default)]
    pub recent_meetings: Vec<RecentMeeting>,
}

fn default_meet_instances() -> Vec<String> {
    vec![
        "meet.linagora.com".to_string(),
        "meet.numerique.gouv.fr".to_string(),
    ]
}

fn default_theme() -> String {
    "light".to_string()
}

fn default_background_mode() -> String {
    "off".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            display_name: None,
            language: None,
            mic_enabled_on_join: true,
            camera_enabled_on_join: false,
            theme: "light".to_string(),
            meet_instances: default_meet_instances(),
            notification_participant_join: true,
            notification_hand_raised: true,
            notification_message_received: true,
            background_mode: "off".to_string(),
            recent_meetings: Vec::new(),
        }
    }
}

pub struct SettingsStore {
    settings: Mutex<Settings>,
    file_path: PathBuf,
}

impl SettingsStore {
    pub fn new(data_dir: &str) -> Self {
        let file_path = PathBuf::from(data_dir).join("settings.json");
        let settings = Self::load(&file_path);
        Self {
            settings: Mutex::new(settings),
            file_path,
        }
    }

    pub fn get(&self) -> Settings {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_display_name(&self, name: Option<String>) {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .display_name = name;
        self.save();
    }

    pub fn set_language(&self, lang: Option<String>) {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .language = lang;
        self.save();
    }

    pub fn set_mic_enabled_on_join(&self, enabled: bool) {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .mic_enabled_on_join = enabled;
        self.save();
    }

    pub fn set_camera_enabled_on_join(&self, enabled: bool) {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .camera_enabled_on_join = enabled;
        self.save();
    }

    pub fn set_theme(&self, theme: String) {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .theme = theme;
        self.save();
    }

    pub fn get_meet_instances(&self) -> Vec<String> {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .meet_instances
            .clone()
    }

    pub fn set_meet_instances(&self, instances: Vec<String>) {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .meet_instances = instances;
        self.save();
    }

    pub fn set_notification_participant_join(&self, enabled: bool) {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .notification_participant_join = enabled;
        self.save();
    }

    pub fn set_notification_hand_raised(&self, enabled: bool) {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .notification_hand_raised = enabled;
        self.save();
    }

    pub fn set_notification_message_received(&self, enabled: bool) {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .notification_message_received = enabled;
        self.save();
    }

    pub fn get_background_mode(&self) -> String {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .background_mode
            .clone()
    }

    pub fn set_background_mode(&self, mode: String) {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .background_mode = mode;
        self.save();
    }

    pub fn get_recent_meetings(&self) -> Vec<RecentMeeting> {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .recent_meetings
            .clone()
    }

    pub fn add_recent_meeting(&self, slug: String, server: String) {
        let mut settings = self.settings.lock().unwrap_or_else(|e| e.into_inner());
        settings
            .recent_meetings
            .retain(|m| !(m.slug == slug && m.server == server));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        settings.recent_meetings.insert(
            0,
            RecentMeeting {
                slug,
                server,
                timestamp_ms: now,
            },
        );
        settings.recent_meetings.truncate(3);
        drop(settings);
        self.save();
    }

    fn save(&self) {
        let settings = self
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if let Some(parent) = self.file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&settings) {
            let _ = std::fs::write(&self.file_path, json);
        }
    }

    fn load(path: &PathBuf) -> Settings {
        match std::fs::read_to_string(path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Settings::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_default_settings() {
        let s = Settings::default();
        assert_eq!(s.display_name, None);
        assert_eq!(s.language, None);
        assert!(s.mic_enabled_on_join);
        assert!(!s.camera_enabled_on_join);
    }

    #[test]
    fn test_new_creates_defaults_when_no_file() {
        let dir = temp_dir();
        let store = SettingsStore::new(dir.path().to_str().unwrap());
        let s = store.get();
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn test_set_display_name_persists() {
        let dir = temp_dir();
        let path = dir.path().to_str().unwrap();
        {
            let store = SettingsStore::new(path);
            store.set_display_name(Some("Alice".to_string()));
        }
        let store = SettingsStore::new(path);
        assert_eq!(store.get().display_name, Some("Alice".to_string()));
    }

    #[test]
    fn test_set_language_persists() {
        let dir = temp_dir();
        let path = dir.path().to_str().unwrap();
        {
            let store = SettingsStore::new(path);
            store.set_language(Some("fr".to_string()));
        }
        let store = SettingsStore::new(path);
        assert_eq!(store.get().language, Some("fr".to_string()));
    }

    #[test]
    fn test_set_mic_camera_persists() {
        let dir = temp_dir();
        let path = dir.path().to_str().unwrap();
        {
            let store = SettingsStore::new(path);
            store.set_mic_enabled_on_join(false);
            store.set_camera_enabled_on_join(true);
        }
        let store = SettingsStore::new(path);
        let s = store.get();
        assert!(!s.mic_enabled_on_join);
        assert!(s.camera_enabled_on_join);
    }

    #[test]
    fn test_clear_display_name() {
        let dir = temp_dir();
        let path = dir.path().to_str().unwrap();
        let store = SettingsStore::new(path);
        store.set_display_name(Some("Bob".to_string()));
        store.set_display_name(None);
        assert_eq!(store.get().display_name, None);
    }

    #[test]
    fn test_corrupt_file_falls_back_to_defaults() {
        let dir = temp_dir();
        let path = dir.path().to_str().unwrap();
        fs::write(dir.path().join("settings.json"), "not json!!!").unwrap();
        let store = SettingsStore::new(path);
        assert_eq!(store.get(), Settings::default());
    }

    #[test]
    fn test_set_theme_persists() {
        let dir = temp_dir();
        let path = dir.path().to_str().unwrap();
        {
            let store = SettingsStore::new(path);
            assert_eq!(store.get().theme, "light");
            store.set_theme("dark".to_string());
        }
        let store = SettingsStore::new(path);
        assert_eq!(store.get().theme, "dark");
    }

    #[test]
    fn test_partial_json_uses_serde_defaults() {
        let dir = temp_dir();
        let path = dir.path().to_str().unwrap();
        fs::write(
            dir.path().join("settings.json"),
            r#"{"display_name":"Eve"}"#,
        )
        .unwrap();
        let store = SettingsStore::new(path);
        let s = store.get();
        assert_eq!(s.display_name, Some("Eve".to_string()));
        assert!(s.mic_enabled_on_join);
        assert!(!s.camera_enabled_on_join);
    }

    #[test]
    fn test_default_meet_instances() {
        let s = Settings::default();
        assert_eq!(
            s.meet_instances,
            vec![
                "meet.linagora.com".to_string(),
                "meet.numerique.gouv.fr".to_string()
            ]
        );
    }

    #[test]
    fn test_set_meet_instances_persists() {
        let dir = temp_dir();
        let path = dir.path().to_str().unwrap();
        {
            let store = SettingsStore::new(path);
            store.set_meet_instances(vec![
                "meet.numerique.gouv.fr".to_string(),
                "meet.example.com".to_string(),
            ]);
        }
        let store = SettingsStore::new(path);
        assert_eq!(
            store.get().meet_instances,
            vec![
                "meet.numerique.gouv.fr".to_string(),
                "meet.example.com".to_string(),
            ]
        );
    }

    #[test]
    fn test_default_notification_settings() {
        let s = Settings::default();
        assert!(s.notification_participant_join);
        assert!(s.notification_hand_raised);
        assert!(s.notification_message_received);
    }

    #[test]
    fn test_set_notification_settings_persist() {
        let dir = temp_dir();
        let path = dir.path().to_str().unwrap();
        {
            let store = SettingsStore::new(path);
            store.set_notification_participant_join(false);
            store.set_notification_hand_raised(false);
            store.set_notification_message_received(false);
        }
        let store = SettingsStore::new(path);
        let s = store.get();
        assert!(!s.notification_participant_join);
        assert!(!s.notification_hand_raised);
        assert!(!s.notification_message_received);
    }

    #[test]
    fn test_background_mode_defaults_to_off() {
        let s = Settings::default();
        assert_eq!(s.background_mode, "off");
    }

    #[test]
    fn test_set_background_mode_persists() {
        let dir = temp_dir();
        let path = dir.path().to_str().unwrap();
        {
            let store = SettingsStore::new(path);
            store.set_background_mode("blur".to_string());
        }
        let store = SettingsStore::new(path);
        assert_eq!(store.get_background_mode(), "blur");
    }

    #[test]
    fn test_set_background_mode_image() {
        let dir = temp_dir();
        let path = dir.path().to_str().unwrap();
        {
            let store = SettingsStore::new(path);
            store.set_background_mode("image:3".to_string());
        }
        let store = SettingsStore::new(path);
        assert_eq!(store.get_background_mode(), "image:3");
    }

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

    #[test]
    fn test_partial_json_defaults_meet_instances() {
        let dir = temp_dir();
        let path = dir.path().to_str().unwrap();
        std::fs::write(
            dir.path().join("settings.json"),
            r#"{"display_name":"Eve"}"#,
        )
        .unwrap();
        let store = SettingsStore::new(path);
        assert_eq!(
            store.get().meet_instances,
            vec![
                "meet.linagora.com".to_string(),
                "meet.numerique.gouv.fr".to_string()
            ]
        );
    }
}
