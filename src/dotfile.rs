use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use crate::git;

const DOTFILE_NAME: &str = ".gswitch";

pub fn find_dotfile() -> Option<PathBuf> {
    // Only search for dotfiles if we're in a git repository
    if !git::is_git_repo() {
        return None;
    }
    
    let git_root = git::find_git_root().ok()?;
    let mut current_dir = std::env::current_dir().ok()?;
    
    // Only search within the git repository boundaries
    loop {
        let dotfile_path = current_dir.join(DOTFILE_NAME);
        if dotfile_path.exists() {
            return Some(dotfile_path);
        }
        
        // Stop if we've reached the git root or can't go up further
        if current_dir == git_root || !current_dir.pop() {
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
    let dotfile_path = find_dotfile()?;
    read_profile_from_dotfile(dotfile_path).ok()
}