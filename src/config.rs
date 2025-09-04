use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitProfile {
    pub name: String,
    pub email: String,
    pub signing_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[derive(Default)]
pub struct Config {
    pub profiles: HashMap<String, GitProfile>,
    pub current_profile: Option<String>,
}


impl Config {
    pub fn config_path() -> Result<PathBuf> {
        // Check for test override first
        if let Ok(test_config_home) = std::env::var("XDG_CONFIG_HOME") {
            let config_dir = std::path::PathBuf::from(test_config_home).join("gswitch");
            return Ok(config_dir.join("config.toml"));
        }
        
        // Use XDG config directory standard for Unix-like systems
        let config_dir = if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(xdg_config_home)
        } else {
            let home = std::env::var("HOME").context("HOME environment variable not set")?;
            std::path::PathBuf::from(home).join(".config")
        };
        
        Ok(config_dir.join("gswitch").join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)
            .context("Failed to read config file")?;
        
        toml::from_str(&content)
            .context("Failed to parse config file")
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        std::fs::write(&config_path, content)
            .context("Failed to write config file")
    }

    pub fn add_profile(&mut self, name: String, profile: GitProfile) {
        self.profiles.insert(name, profile);
    }

    pub fn remove_profile(&mut self, name: &str) -> bool {
        if self.current_profile.as_ref() == Some(&name.to_string()) {
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
        };
        
        config.add_profile("test".to_string(), profile.clone());
        
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
        };
        
        config.add_profile("test".to_string(), profile.clone());
        
        let stored_profile = config.get_profile("test").unwrap();
        assert_eq!(stored_profile.signing_key, Some("ABC123".to_string()));
    }

    #[test]
    fn test_remove_profile() {
        let mut config = Config::default();
        let profile = GitProfile {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            signing_key: None,
        };
        
        config.add_profile("test".to_string(), profile);
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
        };
        
        config.add_profile("test".to_string(), profile);
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
            };
            
            config.add_profile("test".to_string(), profile.clone());
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