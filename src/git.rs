use std::process::Command;
use anyhow::{Context, Result, bail};
use crate::config::GitProfile;

pub fn set_git_config(profile: &GitProfile, global: bool) -> Result<()> {
    let scope = if global { "--global" } else { "--local" };
    
    // Set user name
    let output = Command::new("git")
        .args(["config", scope, "user.name", &profile.name])
        .output()
        .context("Failed to execute git config for user.name")?;
    
    if !output.status.success() {
        bail!("Failed to set git user.name: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Set user email
    let output = Command::new("git")
        .args(["config", scope, "user.email", &profile.email])
        .output()
        .context("Failed to execute git config for user.email")?;
    
    if !output.status.success() {
        bail!("Failed to set git user.email: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Set signing key if provided
    if let Some(signing_key) = &profile.signing_key {
        let output = Command::new("git")
            .args(["config", scope, "user.signingkey", signing_key])
            .output()
            .context("Failed to execute git config for user.signingkey")?;
        
        if !output.status.success() {
            bail!("Failed to set git user.signingkey: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    Ok(())
}

pub fn get_current_git_config() -> Result<GitProfile> {
    let name = get_git_config_value("user.name")?;
    let email = get_git_config_value("user.email")?;
    let signing_key = get_git_config_value("user.signingkey").ok();

    Ok(GitProfile {
        name,
        email,
        signing_key,
    })
}

fn get_git_config_value(key: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["config", "--get", key])
        .output()
        .context(format!("Failed to execute git config --get {}", key))?;
    
    if !output.status.success() {
        bail!("Git config {} not found", key);
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn find_git_root() -> Result<std::path::PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("Failed to execute git rev-parse --show-toplevel")?;
    
    if !output.status.success() {
        bail!("Not in a git repository");
    }

    let root_path = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in git root path")?
        .trim()
        .to_string();
    
    Ok(std::path::PathBuf::from(root_path))
}