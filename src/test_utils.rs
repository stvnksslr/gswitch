#[cfg(test)]
use tempfile::TempDir;
#[cfg(test)]
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::process::Command;
#[cfg(test)]
use anyhow::Result;
#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[cfg(test)]
pub struct TestWorkingDir {
    _temp_dir: TempDir,
    path: PathBuf,
}

#[cfg(test)]
impl TestWorkingDir {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path = temp_dir.path().canonicalize().expect("Failed to canonicalize path");
        
        TestWorkingDir {
            _temp_dir: temp_dir,
            path,
        }
    }
    
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.path.join(path)
    }
    
    pub fn create_file<P: AsRef<Path>>(&self, path: P, content: &str) -> Result<PathBuf> {
        let file_path = self.path.join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file_path, content)?;
        Ok(file_path.canonicalize()?)
    }
    
    pub fn create_dir<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        let dir_path = self.path.join(path);
        std::fs::create_dir_all(&dir_path)?;
        Ok(dir_path.canonicalize()?)
    }
}

#[cfg(test)]
pub struct GitTestRepo {
    work_dir: TestWorkingDir,
}

#[cfg(test)]
impl GitTestRepo {
    pub fn new() -> Self {
        let work_dir = TestWorkingDir::new();
        
        // Initialize git repo
        let output = Command::new("git")
            .args(["init"])
            .current_dir(work_dir.path())
            .output()
            .expect("Failed to initialize git repo");
        
        assert!(output.status.success(), "Git init failed: {}", String::from_utf8_lossy(&output.stderr));
        
        // Set test git config to avoid issues with missing global config
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(work_dir.path())
            .output()
            .expect("Failed to set git user.name");
            
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(work_dir.path())
            .output()
            .expect("Failed to set git user.email");
        
        GitTestRepo { work_dir }
    }
    
    pub fn path(&self) -> &Path {
        self.work_dir.path()
    }
    
    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.work_dir.join(path)
    }
    
    pub fn create_file<P: AsRef<Path>>(&self, path: P, content: &str) -> Result<PathBuf> {
        self.work_dir.create_file(path, content)
    }
    
    pub fn create_dir<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        self.work_dir.create_dir(path)
    }
    
}

#[cfg(test)]
pub fn with_temp_dir<F, R>(f: F) -> R
where
    F: FnOnce(&TestWorkingDir) -> R,
{
    let temp_dir = TestWorkingDir::new();
    f(&temp_dir)
}

#[cfg(test)]
pub fn with_git_repo<F, R>(f: F) -> R
where
    F: FnOnce(&GitTestRepo) -> R,
{
    let git_repo = GitTestRepo::new();
    f(&git_repo)
}

#[cfg(test)]
pub fn with_test_config_env<F, R>(f: F) -> R
where
    F: FnOnce(&Path) -> R,
{
    let _env_lock = ENV_MUTEX.lock().unwrap();
    
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_dir = temp_dir.path().join(".config");
    std::fs::create_dir_all(&config_dir).expect("Failed to create config directory");
    
    let original_config_home = std::env::var("XDG_CONFIG_HOME").ok();
    
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &config_dir);
    }
    
    let result = f(&config_dir);
    
    // Restore original environment
    unsafe {
        if let Some(original) = original_config_home {
            std::env::set_var("XDG_CONFIG_HOME", original);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
    }
    
    result
}

#[cfg(test)]
pub fn canonicalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    path.as_ref().canonicalize().unwrap_or_else(|_| path.as_ref().to_path_buf())
}

#[cfg(test)]
macro_rules! assert_path_eq {
    ($left:expr, $right:expr) => {
        assert_eq!(
            crate::test_utils::canonicalize_path($left),
            crate::test_utils::canonicalize_path($right)
        );
    };
}

#[cfg(test)]
pub(crate) use assert_path_eq;