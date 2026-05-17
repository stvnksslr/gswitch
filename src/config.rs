use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitProfile {
    pub name: String,
    pub email: String,
    pub signing_key: Option<String>,
    /// Signing key format: "gpg", "ssh", or "x509". When set, applied as
    /// `gpg.format`. A missing value (older configs) deserializes to `None`.
    #[serde(default)]
    pub gpg_format: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub profiles: HashMap<String, GitProfile>,
    pub current_profile: Option<String>,
}

/// Validate a profile name
pub fn validate_profile_name(name: &str) -> Result<()> {
    if name.is_empty() || name.trim().is_empty() {
        bail!("Profile name cannot be empty");
    }
    // Check for problematic characters that could cause issues with TOML or file systems
    const INVALID_CHARS: &[char] = &['/', '\\', '\0', '\n', '\r', '\t'];
    if let Some(c) = name.chars().find(|c| INVALID_CHARS.contains(c)) {
        bail!("Profile name contains invalid character: {:?}", c);
    }
    if name.len() > 64 {
        bail!("Profile name is too long (max 64 characters)");
    }
    Ok(())
}

/// Validate an email address (basic validation)
pub fn validate_email(email: &str) -> Result<()> {
    if email.is_empty() || email.trim().is_empty() {
        bail!("Email cannot be empty");
    }
    if !email.contains('@') {
        bail!("Email must contain '@'");
    }
    Ok(())
}

/// Validate a gpg.format value. Git accepts "gpg", "ssh", and "x509".
pub fn validate_gpg_format(format: &str) -> Result<()> {
    match format {
        "gpg" | "ssh" | "x509" => Ok(()),
        _ => bail!(
            "Invalid gpg format: '{}'. Valid values: gpg, ssh, x509",
            format
        ),
    }
}

impl Config {
    pub fn config_path() -> Result<PathBuf> {
        // Check for XDG_CONFIG_HOME (also used for test override)
        let config_dir = if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config_home)
        } else {
            let home = std::env::var("HOME").context("HOME environment variable not set")?;
            PathBuf::from(home).join(".config")
        };

        Ok(config_dir.join("gswitch").join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content =
            std::fs::read_to_string(&config_path).context("Failed to read config file")?;

        toml::from_str(&content).context("Failed to parse config file")
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        // Atomic write: write to temp file then rename
        let temp_path = config_path.with_extension("toml.tmp");
        std::fs::write(&temp_path, &content).context("Failed to write config file")?;
        std::fs::rename(&temp_path, &config_path).context("Failed to finalize config file")?;

        Ok(())
    }

    pub fn add_profile(&mut self, name: String, profile: GitProfile) -> Result<()> {
        validate_profile_name(&name)?;
        validate_email(&profile.email)?;
        if let Some(format) = &profile.gpg_format {
            validate_gpg_format(format)?;
        }
        self.profiles.insert(name, profile);
        Ok(())
    }

    pub fn remove_profile(&mut self, name: &str) -> bool {
        if self.current_profile.as_deref() == Some(name) {
            self.current_profile = None;
        }
        self.profiles.remove(name).is_some()
    }

    pub fn get_profile(&self, name: &str) -> Option<&GitProfile> {
        self.profiles.get(name)
    }

    pub fn set_current_profile(&mut self, name: String) {
        self.current_profile = Some(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.profiles.is_empty());
        assert!(config.current_profile.is_none());
    }

    #[test]
    fn test_add_profile() {
        let mut config = Config::default();
        let profile = GitProfile {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            signing_key: None,
            gpg_format: None,
        };

        config
            .add_profile("test".to_string(), profile.clone())
            .unwrap();

        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.get_profile("test"), Some(&profile));
    }

    #[test]
    fn test_add_profile_with_signing_key() {
        let mut config = Config::default();
        let profile = GitProfile {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            signing_key: Some("ABC123".to_string()),
            gpg_format: None,
        };

        config
            .add_profile("test".to_string(), profile.clone())
            .unwrap();

        let stored_profile = config.get_profile("test").unwrap();
        assert_eq!(stored_profile.signing_key, Some("ABC123".to_string()));
    }

    #[test]
    fn test_add_profile_invalid_name() {
        let mut config = Config::default();
        let profile = GitProfile {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            signing_key: None,
            gpg_format: None,
        };

        // Empty name
        assert!(config.add_profile("".to_string(), profile.clone()).is_err());
        // Name with invalid characters
        assert!(
            config
                .add_profile("test/name".to_string(), profile.clone())
                .is_err()
        );
    }

    #[test]
    fn test_add_profile_invalid_email() {
        let mut config = Config::default();
        let profile = GitProfile {
            name: "Test User".to_string(),
            email: "invalid-email".to_string(),
            signing_key: None,
            gpg_format: None,
        };

        assert!(config.add_profile("test".to_string(), profile).is_err());
    }

    #[test]
    fn test_add_profile_invalid_gpg_format() {
        let mut config = Config::default();
        let profile = GitProfile {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            signing_key: None,
            gpg_format: Some("pgp".to_string()),
        };

        assert!(config.add_profile("test".to_string(), profile).is_err());
    }

    #[test]
    fn test_add_profile_valid_gpg_format() {
        let mut config = Config::default();
        let profile = GitProfile {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            signing_key: Some("ABC123".to_string()),
            gpg_format: Some("ssh".to_string()),
        };

        config.add_profile("test".to_string(), profile).unwrap();
        assert_eq!(
            config.get_profile("test").unwrap().gpg_format,
            Some("ssh".to_string())
        );
    }

    #[test]
    fn test_remove_profile() {
        let mut config = Config::default();
        let profile = GitProfile {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            signing_key: None,
            gpg_format: None,
        };

        config.add_profile("test".to_string(), profile).unwrap();
        assert!(config.remove_profile("test"));
        assert!(config.profiles.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_profile() {
        let mut config = Config::default();
        assert!(!config.remove_profile("nonexistent"));
    }

    #[test]
    fn test_remove_current_profile() {
        let mut config = Config::default();
        let profile = GitProfile {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            signing_key: None,
            gpg_format: None,
        };

        config.add_profile("test".to_string(), profile).unwrap();
        config.set_current_profile("test".to_string());

        assert!(config.remove_profile("test"));
        assert!(config.current_profile.is_none());
    }

    #[test]
    fn test_set_current_profile() {
        let mut config = Config::default();
        config.set_current_profile("test".to_string());
        assert_eq!(config.current_profile, Some("test".to_string()));
    }

    #[test]
    fn test_get_nonexistent_profile() {
        let config = Config::default();
        assert!(config.get_profile("nonexistent").is_none());
    }

    #[test]
    fn test_save_and_load_config() {
        with_test_config_env(|_config_dir| {
            let mut config = Config::default();
            let profile = GitProfile {
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                signing_key: Some("ABC123".to_string()),
                gpg_format: Some("ssh".to_string()),
            };

            config
                .add_profile("test".to_string(), profile.clone())
                .unwrap();
            config.set_current_profile("test".to_string());

            // Save config
            config.save().unwrap();

            // Get the actual config path that was used
            let config_path = Config::config_path().unwrap();
            assert!(config_path.exists());

            // Load config
            let loaded_config = Config::load().unwrap();
            assert_eq!(loaded_config.profiles.len(), 1);
            assert_eq!(loaded_config.get_profile("test"), Some(&profile));
            assert_eq!(loaded_config.current_profile, Some("test".to_string()));
        });
    }

    #[test]
    fn test_load_nonexistent_config() {
        with_test_config_env(|_config_dir| {
            // Should return default config when file doesn't exist
            let config = Config::load().unwrap();
            assert!(config.profiles.is_empty());
            assert!(config.current_profile.is_none());
        });
    }
}
