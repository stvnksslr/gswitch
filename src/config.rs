use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let config_dir = dirs::config_dir()
            .context("Failed to get config directory")?;
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