use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const DOTFILE_NAME: &str = ".gswitch";

pub fn find_dotfile_in_dir<P: AsRef<Path>>(start_dir: Option<P>) -> Option<PathBuf> {
    let mut search_dir = match start_dir {
        Some(dir) => dir.as_ref().to_path_buf(),
        None => std::env::current_dir().ok()?,
    };

    // Walk upwards looking for a `.gswitch` file, stopping at the repository
    // root (the directory containing `.git`). A profile only applies within
    // its own repository, so the search must not cross that boundary; and if
    // no repository root is found at all, there is no profile to apply.
    //
    // This is a pure filesystem walk with no `git` subprocess, which keeps it
    // cheap enough to run on every shell prompt render via `gsw prompt`.
    let mut found: Option<PathBuf> = None;
    loop {
        if found.is_none() {
            let dotfile_path = search_dir.join(DOTFILE_NAME);
            if dotfile_path.exists() {
                found = Some(dotfile_path);
            }
        }

        // `.git` may be a directory (normal repo) or a file (worktree or
        // submodule); `exists()` covers both.
        if search_dir.join(".git").exists() {
            return found;
        }

        if !search_dir.pop() {
            return None;
        }
    }
}

pub fn read_profile_from_dotfile<P: AsRef<Path>>(dotfile_path: P) -> Result<String> {
    let content = std::fs::read_to_string(dotfile_path).context("Failed to read .gswitch file")?;

    let profile_name = content.trim().to_string();

    if profile_name.is_empty() {
        anyhow::bail!(".gswitch file is empty");
    }

    Ok(profile_name)
}

pub fn create_dotfile<P: AsRef<Path>>(path: P, profile_name: &str) -> Result<()> {
    std::fs::write(path, format!("{}\n", profile_name)).context("Failed to create .gswitch file")
}

pub fn get_dotfile_profile() -> Option<String> {
    get_dotfile_profile_in_dir(None::<&Path>)
}

pub fn get_dotfile_profile_in_dir<P: AsRef<Path>>(start_dir: Option<P>) -> Option<String> {
    let dotfile_path = find_dotfile_in_dir(start_dir)?;
    read_profile_from_dotfile(dotfile_path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn test_find_dotfile_in_git_repo() {
        with_git_repo(|repo| {
            // Create .gswitch file
            repo.create_file(".gswitch", "test-profile\n").unwrap();

            let dotfile_path = find_dotfile_in_dir(Some(repo.path()));
            assert!(dotfile_path.is_some());
            assert_path_eq!(dotfile_path.unwrap(), repo.join(".gswitch"));
        });
    }

    #[test]
    fn test_find_dotfile_in_subdirectory() {
        with_git_repo(|repo| {
            // Create .gswitch file in root
            repo.create_file(".gswitch", "test-profile\n").unwrap();

            // Create subdirectory
            let subdir = repo.create_dir("subdir").unwrap();

            // Should find .gswitch file in parent (git root) when searching from subdirectory
            let dotfile_path = find_dotfile_in_dir(Some(&subdir));
            assert!(dotfile_path.is_some());
            assert_path_eq!(dotfile_path.unwrap(), repo.join(".gswitch"));
        });
    }

    #[test]
    fn test_find_dotfile_stops_at_repo_root() {
        with_temp_dir(|outer| {
            // A `.gswitch` above the repository must never be picked up.
            outer.create_file(".gswitch", "outside-profile\n").unwrap();

            let repo_dir = outer.create_dir("repo").unwrap();
            std::process::Command::new("git")
                .args(["init"])
                .current_dir(&repo_dir)
                .output()
                .expect("Failed to initialize git repo");

            // Searching from the repo root must not cross into the parent dir.
            assert!(find_dotfile_in_dir(Some(&repo_dir)).is_none());
        });
    }

    #[test]
    fn test_find_dotfile_not_in_git_repo() {
        with_temp_dir(|temp_dir| {
            // Create .gswitch file in non-git directory
            temp_dir.create_file(".gswitch", "test-profile\n").unwrap();

            // Should not find .gswitch file because not in git repo
            let dotfile_path = find_dotfile_in_dir(Some(temp_dir.path()));
            assert!(dotfile_path.is_none());
        });
    }

    #[test]
    fn test_find_dotfile_no_file() {
        with_git_repo(|repo| {
            // No .gswitch file in git repo
            let dotfile_path = find_dotfile_in_dir(Some(repo.path()));
            assert!(dotfile_path.is_none());
        });
    }

    #[test]
    fn test_read_profile_from_dotfile() {
        with_temp_dir(|temp_dir| {
            let gswitch_path = temp_dir.create_file(".gswitch", "work-profile\n").unwrap();

            let profile_name = read_profile_from_dotfile(&gswitch_path).unwrap();
            assert_eq!(profile_name, "work-profile");
        });
    }

    #[test]
    fn test_read_profile_from_dotfile_with_whitespace() {
        with_temp_dir(|temp_dir| {
            let gswitch_path = temp_dir
                .create_file(".gswitch", "  work-profile  \n")
                .unwrap();

            let profile_name = read_profile_from_dotfile(&gswitch_path).unwrap();
            assert_eq!(profile_name, "work-profile");
        });
    }

    #[test]
    fn test_read_profile_from_empty_dotfile() {
        with_temp_dir(|temp_dir| {
            let gswitch_path = temp_dir.create_file(".gswitch", "").unwrap();

            let result = read_profile_from_dotfile(&gswitch_path);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("empty"));
        });
    }

    #[test]
    fn test_read_profile_from_whitespace_only_dotfile() {
        with_temp_dir(|temp_dir| {
            let gswitch_path = temp_dir.create_file(".gswitch", "   \n  \t  \n").unwrap();

            let result = read_profile_from_dotfile(&gswitch_path);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("empty"));
        });
    }

    #[test]
    fn test_read_profile_from_nonexistent_dotfile() {
        with_temp_dir(|temp_dir| {
            let gswitch_path = temp_dir.join(".gswitch");

            let result = read_profile_from_dotfile(&gswitch_path);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_create_dotfile() {
        with_temp_dir(|temp_dir| {
            let gswitch_path = temp_dir.join(".gswitch");

            create_dotfile(&gswitch_path, "test-profile").unwrap();

            assert!(gswitch_path.exists());
            let content = std::fs::read_to_string(&gswitch_path).unwrap();
            assert_eq!(content, "test-profile\n");
        });
    }

    #[test]
    fn test_get_dotfile_profile() {
        with_git_repo(|repo| {
            // Create .gswitch file
            repo.create_file(".gswitch", "integration-test\n").unwrap();

            let profile_name = get_dotfile_profile_in_dir(Some(repo.path()));
            assert_eq!(profile_name, Some("integration-test".to_string()));
        });
    }

    #[test]
    fn test_get_dotfile_profile_no_file() {
        with_git_repo(|repo| {
            // No .gswitch file in git repo
            let profile_name = get_dotfile_profile_in_dir(Some(repo.path()));
            assert!(profile_name.is_none());
        });
    }

    #[test]
    fn test_get_dotfile_profile_not_in_git_repo() {
        with_temp_dir(|temp_dir| {
            // Create .gswitch file in non-git directory
            temp_dir
                .create_file(".gswitch", "should-not-find\n")
                .unwrap();

            // Should return None because not in git repo
            let profile_name = get_dotfile_profile_in_dir(Some(temp_dir.path()));
            assert!(profile_name.is_none());
        });
    }
}
