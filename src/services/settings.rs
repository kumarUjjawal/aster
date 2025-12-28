use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Application settings with persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Font size in points (8-32, default 14)
    #[serde(default = "default_font_size")]
    pub font_size: f32,
}

fn default_font_size() -> f32 {
    14.0
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            font_size: default_font_size(),
        }
    }
}

impl Settings {
    /// Minimum allowed font size
    pub const MIN_FONT_SIZE: f32 = 8.0;
    /// Maximum allowed font size
    pub const MAX_FONT_SIZE: f32 = 32.0;
    /// Default font size
    pub const DEFAULT_FONT_SIZE: f32 = 14.0;
    /// Font size step for increase/decrease
    pub const FONT_SIZE_STEP: f32 = 2.0;

    /// Clamp font size to valid range
    pub fn clamp_font_size(size: f32) -> f32 {
        size.clamp(Self::MIN_FONT_SIZE, Self::MAX_FONT_SIZE)
    }
}

/// Global settings manager with lazy loading and auto-save
pub struct SettingsManager {
    settings: Settings,
    path: Option<PathBuf>,
}

impl SettingsManager {
    /// Load settings from disk or create defaults
    pub fn load() -> Self {
        let path = Self::settings_path();
        let settings = path
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Self { settings, path }
    }

    /// Get current settings
    pub fn get(&self) -> &Settings {
        &self.settings
    }

    /// Update settings and persist to disk
    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Settings),
    {
        f(&mut self.settings);
        self.save();
    }

    /// Save settings to disk
    fn save(&self) {
        let Some(ref path) = self.path else { return };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Write atomically via temp file
        if let Ok(json) = serde_json::to_string_pretty(&self.settings) {
            let _ = fs::write(path, json);
        }
    }

    /// Get settings file path
    fn settings_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "kumarujjawal", "aster")
            .map(|dirs| dirs.config_dir().join("settings.json"))
    }
}

/// Thread-safe global settings instance
static SETTINGS: once_cell::sync::Lazy<Arc<Mutex<SettingsManager>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(SettingsManager::load())));

/// Get the global settings manager
pub fn settings() -> Arc<Mutex<SettingsManager>> {
    SETTINGS.clone()
}

/// Convenience function to get current font size
pub fn get_font_size() -> f32 {
    settings()
        .lock()
        .map(|s| s.get().font_size)
        .unwrap_or(Settings::DEFAULT_FONT_SIZE)
}

/// Convenience function to set font size
pub fn set_font_size(size: f32) {
    let clamped = Settings::clamp_font_size(size);
    if let Ok(mut manager) = settings().lock() {
        manager.update(|s| s.font_size = clamped);
    }
}
