use crate::config::GitProfile;
use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

/// Helper to set a single git config value
fn set_git_config_value(scope: &str, key: &str, value: &str, dir: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["config", scope, key, value]);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let output = cmd
        .output()
        .with_context(|| format!("Failed to execute git config for {}", key))?;

    if !output.status.success() {
        bail!(
            "Failed to set git {}: {}",
            key,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// Helper to unset a single git config value.
///
/// `git config --unset` exits with code 5 when the key is not present; that is
/// expected and treated as success so switching to a profile that omits a value
/// (e.g. a signing key) reliably clears any stale value left by a prior profile.
fn unset_git_config_value(scope: &str, key: &str, dir: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["config", scope, "--unset", key]);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let output = cmd
        .output()
        .with_context(|| format!("Failed to execute git config --unset for {}", key))?;

    // Exit code 5 == "key was not set" — nothing to unset, so this is fine.
    if !output.status.success() && output.status.code() != Some(5) {
        bail!(
            "Failed to unset git {}: {}",
            key,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub fn set_git_config(profile: &GitProfile, global: bool) -> Result<()> {
    set_git_config_in_dir(profile, global, None::<&Path>)
}

pub fn set_git_config_in_dir<P: AsRef<Path>>(
    profile: &GitProfile,
    global: bool,
    dir: Option<P>,
) -> Result<()> {
    let scope = if global { "--global" } else { "--local" };
    let dir_ref = dir.as_ref().map(|d| d.as_ref());

    set_git_config_value(scope, "user.name", &profile.name, dir_ref)?;
    set_git_config_value(scope, "user.email", &profile.email, dir_ref)?;

    // Set or clear the signing key so a profile without one does not inherit a
    // stale key from a previously applied profile.
    if let Some(signing_key) = &profile.signing_key {
        set_git_config_value(scope, "user.signingkey", signing_key, dir_ref)?;
    } else {
        unset_git_config_value(scope, "user.signingkey", dir_ref)?;
    }

    // Likewise for gpg.format (gpg / ssh / x509). Clearing it lets git fall back
    // to its default rather than signing with the wrong key type.
    if let Some(gpg_format) = &profile.gpg_format {
        set_git_config_value(scope, "gpg.format", gpg_format, dir_ref)?;
    } else {
        unset_git_config_value(scope, "gpg.format", dir_ref)?;
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
    let gpg_format = get_git_config_value_in_dir("gpg.format", dir.as_ref()).ok();

    Ok(GitProfile {
        name,
        email,
        signing_key,
        gpg_format,
    })
}

fn get_git_config_value_in_dir<P: AsRef<Path>>(key: &str, dir: Option<P>) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.args(["config", "--get", key]);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let output = cmd
        .output()
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
    fn test_get_git_repo_info() {
        with_git_repo(|repo| {
            // Create subdirectory
            let subdir = repo.create_dir("subdir").unwrap();

            // Should find git root from subdirectory
            let git_root = get_git_repo_info(Some(&subdir)).unwrap();
            assert_path_eq!(git_root, repo.path());
        });
    }

    #[test]
    fn test_get_git_repo_info_not_in_git_repo() {
        with_temp_dir(|temp_dir| {
            // Should return None in non-git directory
            assert!(get_git_repo_info(Some(temp_dir.path())).is_none());
        });
    }

    #[test]
    fn test_set_and_get_git_config() {
        with_git_repo(|repo| {
            let profile = GitProfile {
                name: "Test User Local".to_string(),
                email: "test-local@example.com".to_string(),
                signing_key: Some("ABC123".to_string()),
                gpg_format: Some("ssh".to_string()),
            };

            // Set git config locally
            set_git_config_in_dir(&profile, false, Some(repo.path())).unwrap();

            // Get current git config
            let current_profile = get_current_git_config_in_dir(Some(repo.path())).unwrap();
            assert_eq!(current_profile.name, "Test User Local");
            assert_eq!(current_profile.email, "test-local@example.com");
            assert_eq!(current_profile.signing_key, Some("ABC123".to_string()));
            assert_eq!(current_profile.gpg_format, Some("ssh".to_string()));
        });
    }

    #[test]
    fn test_set_git_config_without_signing_key() {
        with_git_repo(|repo| {
            let profile = GitProfile {
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                signing_key: None,
                gpg_format: None,
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
    fn test_switching_profile_clears_stale_signing_key() {
        with_git_repo(|repo| {
            // First profile has a signing key and an explicit gpg format.
            let signed = GitProfile {
                name: "Signed User".to_string(),
                email: "signed@example.com".to_string(),
                signing_key: Some("ABC123".to_string()),
                gpg_format: Some("ssh".to_string()),
            };
            set_git_config_in_dir(&signed, false, Some(repo.path())).unwrap();

            // Switching to a profile without a key must clear the stale values.
            let unsigned = GitProfile {
                name: "Unsigned User".to_string(),
                email: "unsigned@example.com".to_string(),
                signing_key: None,
                gpg_format: None,
            };
            set_git_config_in_dir(&unsigned, false, Some(repo.path())).unwrap();

            assert_eq!(
                get_current_git_config_in_dir(Some(repo.path())).unwrap().name,
                "Unsigned User"
            );

            // Check the *local* scope directly: get_current_git_config reads the
            // effective config, which may fall back to a global signing key.
            let local_value = |key: &str| {
                Command::new("git")
                    .args(["config", "--local", "--get", key])
                    .current_dir(repo.path())
                    .output()
                    .unwrap()
            };
            assert!(
                !local_value("user.signingkey").status.success(),
                "local user.signingkey should have been unset"
            );
            assert!(
                !local_value("gpg.format").status.success(),
                "local gpg.format should have been unset"
            );
        });
    }

    #[test]
    fn test_get_git_config_value_missing() {
        with_git_repo(|repo| {
            // Should fail to get a config value that definitely doesn't exist
            assert!(
                get_git_config_value_in_dir("nonexistent.config.key", Some(repo.path())).is_err()
            );
        });
    }
}
