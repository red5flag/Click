use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Configuration error types
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("Configuration file not found at: {0}")]
    NotFound(String),
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Camera device index
    pub camera_index: i32,
    /// Confidence threshold for person detection (0-1)
    pub confidence_threshold: f32,
    /// IoU threshold for NMS (0-1)
    pub iou_threshold: f32,
    /// Grace period after person leaves (seconds)
    pub grace_period_seconds: u64,
    /// Directory to save recordings
    pub recordings_directory: String,
    /// Maximum channel depth for bounded queues
    pub max_channel_depth: usize,
    /// Model file path
    pub model_path: String,
    /// Video codec preference (h264, h265, or auto)
    pub video_codec: String,
    /// Target FPS for recording
    pub recording_fps: f64,
    /// Performance metrics logging interval (seconds)
    pub metrics_interval_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            camera_index: 0,
            confidence_threshold: 0.5,
            iou_threshold: 0.45,
            grace_period_seconds: 10,
            recordings_directory: "./recordings".to_string(),
            max_channel_depth: 3,
            model_path: "./models/yolo12n.onnx".to_string(),
            video_codec: "h264".to_string(),
            recording_fps: 30.0,
            metrics_interval_seconds: 30,
        }
    }
}

impl Config {
    /// Load configuration from file or create default
    pub fn load() -> Result<Self, ConfigError> {
        let config_paths = [
            "./config.toml",
            "/etc/person-detect/config.toml",
            "~/.config/person-detect/config.toml",
        ];

        for path in &config_paths {
            let expanded_path = shellexpand::tilde(path);
            let path_obj = Path::new(expanded_path.as_ref());
            
            if path_obj.exists() {
                let contents = fs::read_to_string(path_obj)?;
                let config: Config = toml::from_str(&contents)?;
                config.validate()?;
                return Ok(config);
            }
        }

        // No config file found, use defaults
        Ok(Config::default())
    }

    /// Load from a specific file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(ConfigError::NotFound(path.display().to_string()));
        }

        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let toml_string = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::Invalid(e.to_string()))?;
        fs::write(path, toml_string)?;
        Ok(())
    }

    /// Validate configuration values
    fn validate(&self) -> Result<(), ConfigError> {
        if !(0.0..=1.0).contains(&self.confidence_threshold) {
            return Err(ConfigError::Invalid(
                format!("confidence_threshold must be between 0 and 1, got {}", self.confidence_threshold)
            ));
        }

        if !(0.0..=1.0).contains(&self.iou_threshold) {
            return Err(ConfigError::Invalid(
                format!("iou_threshold must be between 0 and 1, got {}", self.iou_threshold)
            ));
        }

        if self.grace_period_seconds == 0 {
            return Err(ConfigError::Invalid(
                "grace_period_seconds must be greater than 0".to_string()
            ));
        }

        if self.max_channel_depth == 0 {
            return Err(ConfigError::Invalid(
                "max_channel_depth must be greater than 0".to_string()
            ));
        }

        Ok(())
    }

    /// Get recordings directory as Path
    pub fn recordings_dir(&self) -> &Path {
        Path::new(&self.recordings_directory)
    }

    /// Get model path as Path
    pub fn model_path(&self) -> &Path {
        Path::new(&self.model_path)
    }
}

// Simple shell expansion for tilde
mod shellexpand {
    pub fn tilde(path: &str) -> std::borrow::Cow<'_, str> {
        if path.starts_with("~/") {
            if let Ok(home) = std::env::var("HOME") {
                return std::borrow::Cow::Owned(format!("{}{}", home, &path[1..]));
            }
        }
        std::borrow::Cow::Borrowed(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.camera_index, 0);
        assert_eq!(config.confidence_threshold, 0.5);
        assert_eq!(config.iou_threshold, 0.45);
        assert_eq!(config.grace_period_seconds, 10);
        assert_eq!(config.recordings_directory, "./recordings");
        assert_eq!(config.max_channel_depth, 3);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());

        // Invalid confidence threshold
        config.confidence_threshold = 1.5;
        assert!(config.validate().is_err());

        config.confidence_threshold = 0.5;
        config.iou_threshold = -0.1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        let original = Config::default();
        original.save(&config_path).unwrap();

        let loaded = Config::from_file(&config_path).unwrap();
        assert_eq!(original.camera_index, loaded.camera_index);
        assert_eq!(original.confidence_threshold, loaded.confidence_threshold);
        assert_eq!(original.iou_threshold, loaded.iou_threshold);
    }

    #[test]
    fn test_toml_parsing() {
        let toml_str = r#"
camera_index = 1
confidence_threshold = 0.6
iou_threshold = 0.5
grace_period_seconds = 15
recordings_directory = "/var/recordings"
max_channel_depth = 5
model_path = "/opt/models/yolo.onnx"
video_codec = "h265"
recording_fps = 25.0
metrics_interval_seconds = 60
"#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.camera_index, 1);
        assert_eq!(config.confidence_threshold, 0.6);
        assert_eq!(config.iou_threshold, 0.5);
        assert_eq!(config.grace_period_seconds, 15);
        assert_eq!(config.recordings_directory, "/var/recordings");
        assert_eq!(config.max_channel_depth, 5);
        assert_eq!(config.model_path, "/opt/models/yolo.onnx");
        assert_eq!(config.video_codec, "h265");
        assert_eq!(config.recording_fps, 25.0);
        assert_eq!(config.metrics_interval_seconds, 60);
    }
}
