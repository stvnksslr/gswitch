use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use crate::git;

const DOTFILE_NAME: &str = ".gswitch";


pub fn find_dotfile_in_dir<P: AsRef<Path>>(start_dir: Option<P>) -> Option<PathBuf> {
    let current_dir = if let Some(dir) = start_dir {
        dir.as_ref().to_path_buf()
    } else {
        std::env::current_dir().ok()?
    };
    
    // Early exit: Check if .gswitch exists in current directory first (most common case)
    let dotfile_path = current_dir.join(DOTFILE_NAME);
    if dotfile_path.exists() {
        // Still need to verify we're in a git repo for the file to be valid
        if git::get_git_repo_info(Some(&current_dir)).is_some() {
            return Some(dotfile_path);
        }
    }
    
    // Combined git check and root finding in one call
    let git_root = git::get_git_repo_info(Some(&current_dir))?;
    let mut search_dir = current_dir;
    
    // Only search within the git repository boundaries
    loop {
        let dotfile_path = search_dir.join(DOTFILE_NAME);
        if dotfile_path.exists() {
            return Some(dotfile_path);
        }
        
        // Stop if we've reached the git root or can't go up further
        if search_dir == git_root || !search_dir.pop() {
            break;
        }
    }
    
    None
}

pub fn read_profile_from_dotfile<P: AsRef<Path>>(dotfile_path: P) -> Result<String> {
    let content = std::fs::read_to_string(dotfile_path)
        .context("Failed to read .gswitch file")?;
    
    let profile_name = content.trim().to_string();
    
    if profile_name.is_empty() {
        anyhow::bail!(".gswitch file is empty");
    }
    
    Ok(profile_name)
}

pub fn create_dotfile<P: AsRef<Path>>(path: P, profile_name: &str) -> Result<()> {
    std::fs::write(path, format!("{}\n", profile_name))
        .context("Failed to create .gswitch file")
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
            let gswitch_path = temp_dir.create_file(".gswitch", "  work-profile  \n").unwrap();
            
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
            temp_dir.create_file(".gswitch", "should-not-find\n").unwrap();
            
            // Should return None because not in git repo
            let profile_name = get_dotfile_profile_in_dir(Some(temp_dir.path()));
            assert!(profile_name.is_none());
        });
    }
}