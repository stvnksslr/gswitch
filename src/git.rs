use std::process::Command;
use std::path::Path;
use anyhow::{Context, Result, bail};
use crate::config::GitProfile;

pub fn set_git_config(profile: &GitProfile, global: bool) -> Result<()> {
    set_git_config_in_dir(profile, global, None::<&Path>)
}

pub fn set_git_config_in_dir<P: AsRef<Path>>(profile: &GitProfile, global: bool, dir: Option<P>) -> Result<()> {
    let scope = if global { "--global" } else { "--local" };
    
    // Set user name
    let mut cmd = Command::new("git");
    cmd.args(["config", scope, "user.name", &profile.name]);
    if let Some(d) = &dir {
        cmd.current_dir(d);
    }
    let output = cmd.output()
        .context("Failed to execute git config for user.name")?;
    
    if !output.status.success() {
        bail!("Failed to set git user.name: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Set user email
    let mut cmd = Command::new("git");
    cmd.args(["config", scope, "user.email", &profile.email]);
    if let Some(d) = &dir {
        cmd.current_dir(d);
    }
    let output = cmd.output()
        .context("Failed to execute git config for user.email")?;
    
    if !output.status.success() {
        bail!("Failed to set git user.email: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Set signing key if provided
    if let Some(signing_key) = &profile.signing_key {
        let mut cmd = Command::new("git");
        cmd.args(["config", scope, "user.signingkey", signing_key]);
        if let Some(d) = &dir {
            cmd.current_dir(d);
        }
        let output = cmd.output()
            .context("Failed to execute git config for user.signingkey")?;
        
        if !output.status.success() {
            bail!("Failed to set git user.signingkey: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    Ok(())
}

pub fn get_current_git_config() -> Result<GitProfile> {
    get_current_git_config_in_dir(None::<&Path>)
}

pub fn get_current_git_config_in_dir<P: AsRef<Path>>(dir: Option<P>) -> Result<GitProfile> {
    let name = get_git_config_value_in_dir("user.name", dir.as_ref())?;
    let email = get_git_config_value_in_dir("user.email", dir.as_ref())?;
    let signing_key = get_git_config_value_in_dir("user.signingkey", dir.as_ref()).ok();

    Ok(GitProfile {
        name,
        email,
        signing_key,
    })
}


fn get_git_config_value_in_dir<P: AsRef<Path>>(key: &str, dir: Option<P>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.args(["config", "--get", key]);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let output = cmd.output()
        .context(format!("Failed to execute git config --get {}", key))?;
    
    if !output.status.success() {
        bail!("Git config {} not found", key);
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

pub fn is_git_repo() -> bool {
    is_git_repo_in_dir(None::<&Path>)
}

pub fn is_git_repo_in_dir<P: AsRef<Path>>(dir: Option<P>) -> bool {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "--show-toplevel"]);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    cmd.output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn find_git_root_in_dir<P: AsRef<Path>>(dir: Option<P>) -> Result<std::path::PathBuf> {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "--show-toplevel"]);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let output = cmd.output()
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

/// Combined function to check if in git repo and get root - more efficient than separate calls
pub fn get_git_repo_info<P: AsRef<Path>>(dir: Option<P>) -> Option<std::path::PathBuf> {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "--show-toplevel"]);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    
    cmd.output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|root| std::path::PathBuf::from(root.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_is_git_repo_in_git_directory() {
        with_git_repo(|repo| {
            assert!(is_git_repo_in_dir(Some(repo.path())));
        });
    }

    #[test]
    fn test_is_git_repo_not_in_git_directory() {
        with_temp_dir(|temp_dir| {
            assert!(!is_git_repo_in_dir(Some(temp_dir.path())));
        });
    }

    #[test]
    fn test_find_git_root() {
        with_git_repo(|repo| {
            // Create subdirectory
            let subdir = repo.create_dir("subdir").unwrap();
            
            // Should find git root from subdirectory
            let git_root = find_git_root_in_dir(Some(&subdir)).unwrap();
            assert_path_eq!(git_root, repo.path());
        });
    }

    #[test]
    fn test_find_git_root_not_in_git_repo() {
        with_temp_dir(|temp_dir| {
            // Should fail to find git root in non-git directory
            assert!(find_git_root_in_dir(Some(temp_dir.path())).is_err());
        });
    }

    #[test]
    fn test_set_and_get_git_config() {
        with_git_repo(|repo| {
            let profile = GitProfile {
                name: "Test User Local".to_string(),
                email: "test-local@example.com".to_string(),
                signing_key: Some("ABC123".to_string()),
            };
            
            // Set git config locally
            set_git_config_in_dir(&profile, false, Some(repo.path())).unwrap();
            
            // Get current git config
            let current_profile = get_current_git_config_in_dir(Some(repo.path())).unwrap();
            assert_eq!(current_profile.name, "Test User Local");
            assert_eq!(current_profile.email, "test-local@example.com");
            assert_eq!(current_profile.signing_key, Some("ABC123".to_string()));
        });
    }

    #[test]
    fn test_set_git_config_without_signing_key() {
        with_git_repo(|repo| {
            let profile = GitProfile {
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                signing_key: None,
            };
            
            // Set git config locally
            set_git_config_in_dir(&profile, false, Some(repo.path())).unwrap();
            
            // Get current git config
            let current_profile = get_current_git_config_in_dir(Some(repo.path())).unwrap();
            assert_eq!(current_profile.name, "Test User");
            assert_eq!(current_profile.email, "test@example.com");
            // signing_key might be None or not present
        });
    }

    #[test]
    fn test_get_git_config_value_missing() {
        with_git_repo(|repo| {
            // Should fail to get a config value that definitely doesn't exist
            assert!(get_git_config_value_in_dir("nonexistent.config.key", Some(repo.path())).is_err());
        });
    }
}