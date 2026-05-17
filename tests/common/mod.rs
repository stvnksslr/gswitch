use assert_cmd::Command;
use tempfile::TempDir;

pub struct TestEnv {
    pub temp_dir: TempDir,
    config_home: String,
}

impl TestEnv {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config_home = temp_dir
            .path()
            .join(".config")
            .to_string_lossy()
            .to_string();
        let config_dir = temp_dir.path().join(".config/gswitch");

        std::fs::create_dir_all(&config_dir).expect("Failed to create config directory");

        // Config home will be passed via environment variables

        TestEnv {
            temp_dir,
            config_home,
        }
    }

    pub fn command(&self) -> Command {
        let mut cmd = Command::cargo_bin("gsw").expect("Failed to find gsw binary");
        cmd.env("XDG_CONFIG_HOME", &self.config_home);
        cmd.current_dir(self.temp_dir.path());
        cmd
    }

    pub fn create_gswitch_file<P: AsRef<std::path::Path>>(&self, path: P, content: &str) {
        let full_path = self.temp_dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent directories");
        }
        std::fs::write(full_path, content).expect("Failed to write .gswitch file");
    }

    pub fn change_to_temp_dir(&self) {
        std::env::set_current_dir(self.temp_dir.path())
            .expect("Failed to change to temp directory");
    }

    /// Initialize the temp directory as a git repository. Required for
    /// commands that resolve `.gswitch` files, since resolution is bounded by
    /// the enclosing repository root.
    pub fn init_git_repo(&self) {
        let output = std::process::Command::new("git")
            .args(["init"])
            .current_dir(self.temp_dir.path())
            .output()
            .expect("Failed to run git init");
        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
