mod common;

use predicates::prelude::*;
use common::TestEnv;

#[test]
fn test_list_no_profiles() {
    let test_env = TestEnv::new();
    
    let mut cmd = test_env.command();
    cmd.arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No profiles configured"));
}

#[test]
fn test_add_and_list_profile() {
    let test_env = TestEnv::new();
    
    // Add a profile
    let mut cmd = test_env.command();
    cmd.args(["add", "test", "--user-name", "Test User", "--email", "test@example.com"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Profile 'test' added successfully"));
    
    // List profiles
    let mut cmd = test_env.command();
    cmd.arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("test - Test User <test@example.com>"));
}

#[test]
fn test_add_profile_with_signing_key() {
    let test_env = TestEnv::new();
    
    let mut cmd = test_env.command();
    cmd.args([
        "add", "test-key", 
        "--user-name", "Test User", 
        "--email", "test@example.com",
        "--signing-key", "ABC123"
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Profile 'test-key' added successfully"));
    
    // List should show the signing key
    let mut cmd = test_env.command();
    cmd.arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Signing key: ABC123"));
}

#[test]
fn test_remove_profile() {
    let test_env = TestEnv::new();
    
    // Add a profile first
    let mut cmd = test_env.command();
    cmd.args(["add", "test", "--user-name", "Test User", "--email", "test@example.com"]);
    cmd.assert().success();
    
    // Remove the profile
    let mut cmd = test_env.command();
    cmd.args(["remove", "test"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Profile 'test' removed successfully"));
    
    // List should be empty
    let mut cmd = test_env.command();
    cmd.arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No profiles configured"));
}

#[test]
fn test_remove_nonexistent_profile() {
    let test_env = TestEnv::new();
    
    let mut cmd = test_env.command();
    cmd.args(["remove", "nonexistent"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Profile 'nonexistent' not found"));
}

#[test]
fn test_init_with_valid_profile() {
    let test_env = TestEnv::new();
    test_env.change_to_temp_dir();
    
    // Add a profile first
    let mut cmd = test_env.command();
    cmd.args(["add", "test", "--user-name", "Test User", "--email", "test@example.com"]);
    cmd.assert().success();
    
    // Initialize .gswitch file
    let mut cmd = test_env.command();
    cmd.args(["init", "test"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created .gswitch file with profile 'test'"));
    
    // Check that .gswitch file was created with correct content
    let gswitch_path = test_env.temp_dir.path().join(".gswitch");
    assert!(gswitch_path.exists(), "File should exist at: {:?}", gswitch_path);
    let content = std::fs::read_to_string(&gswitch_path).unwrap();
    assert_eq!(content.trim(), "test");
}

#[test]
fn test_init_with_invalid_profile() {
    let test_env = TestEnv::new();
    test_env.change_to_temp_dir();
    
    let mut cmd = test_env.command();
    cmd.args(["init", "nonexistent"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Profile 'nonexistent' not found"));
}

#[test]
fn test_prompt_with_gswitch_file() {
    let test_env = TestEnv::new();
    test_env.change_to_temp_dir();
    test_env.create_gswitch_file(".gswitch", "test-profile");
    
    let mut cmd = test_env.command();
    cmd.arg("prompt");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("test-profile"));
}

#[test]
fn test_prompt_without_gswitch_file() {
    let test_env = TestEnv::new();
    test_env.change_to_temp_dir();
    
    let mut cmd = test_env.command();
    cmd.arg("prompt");
    cmd.assert()
        .failure() // Should exit with code 1
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_prompt_with_empty_gswitch_file() {
    let test_env = TestEnv::new();
    test_env.change_to_temp_dir();
    test_env.create_gswitch_file(".gswitch", "");
    
    let mut cmd = test_env.command();
    cmd.arg("prompt");
    cmd.assert()
        .failure() // Should exit with code 1
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_prompt_with_whitespace_only_gswitch_file() {
    let test_env = TestEnv::new();
    test_env.change_to_temp_dir();
    test_env.create_gswitch_file(".gswitch", "   \n  \t  ");
    
    let mut cmd = test_env.command();
    cmd.arg("prompt");
    cmd.assert()
        .failure() // Should exit with code 1
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_activate_bash() {
    let test_env = TestEnv::new();
    
    let mut cmd = test_env.command();
    cmd.args(["activate", "bash"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("_gsw_auto_switch()"))
        .stdout(predicate::str::contains("gsw auto"));
}

#[test]
fn test_activate_zsh() {
    let test_env = TestEnv::new();
    
    let mut cmd = test_env.command();
    cmd.args(["activate", "zsh"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("add-zsh-hook"))
        .stdout(predicate::str::contains("chpwd"));
}

#[test]
fn test_activate_fish() {
    let test_env = TestEnv::new();
    
    let mut cmd = test_env.command();
    cmd.args(["activate", "fish"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--on-variable PWD"))
        .stdout(predicate::str::contains("gsw auto"));
}

#[test]
fn test_activate_nushell() {
    let test_env = TestEnv::new();
    
    let mut cmd = test_env.command();
    cmd.args(["activate", "nushell"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("env_change"))
        .stdout(predicate::str::contains("PWD"));
}

#[test]
fn test_activate_unsupported_shell() {
    let test_env = TestEnv::new();
    
    let mut cmd = test_env.command();
    cmd.args(["activate", "unsupported"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Unsupported shell: unsupported"));
}

#[test]
fn test_current_format_name() {
    let test_env = TestEnv::new();
    
    let mut cmd = test_env.command();
    cmd.args(["current", "--format", "name"]);
    // This might fail if no git config is set, but should not crash
    cmd.assert().code(predicate::in_iter([0, 1]));
}

#[test]
fn test_current_format_email() {
    let test_env = TestEnv::new();
    
    let mut cmd = test_env.command();
    cmd.args(["current", "--format", "email"]);
    // This might fail if no git config is set, but should not crash
    cmd.assert().code(predicate::in_iter([0, 1]));
}

#[test]
fn test_current_invalid_format() {
    let test_env = TestEnv::new();
    
    let mut cmd = test_env.command();
    cmd.args(["current", "--format", "invalid"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Invalid format: invalid"));
}