mod config;
mod git;
mod dotfile;

#[cfg(test)]
mod test_utils;

use clap::{Parser, Subcommand};
use anyhow::Result;
use config::{Config, GitProfile};

#[derive(Parser)]
#[command(name = "gsw")]
#[command(about = "A CLI tool for switching git profiles")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new git profile
    Add {
        /// Profile name
        name: String,
        /// Git user name
        #[arg(long)]
        user_name: String,
        /// Git user email
        #[arg(long)]
        email: String,
        /// Git signing key (optional)
        #[arg(long)]
        signing_key: Option<String>,
    },
    /// List all profiles
    List,
    /// Remove a profile
    Remove {
        /// Profile name to remove
        name: String,
    },
    /// Switch to a profile globally
    Switch {
        /// Profile name to switch to
        name: String,
    },
    /// Switch to a profile locally (current repo only)
    Local {
        /// Profile name to switch to
        name: String,
    },
    /// Show current git configuration
    Current {
        /// Output format (full, name, email)
        #[arg(long, default_value = "full")]
        format: String,
    },
    /// Auto-switch based on .gswitch file
    Auto,
    /// Create a .gswitch file in current directory
    Init {
        /// Profile name to set in .gswitch file
        profile: String,
    },
    /// Import current git identity as a new profile
    Import {
        /// Profile name for the imported identity
        name: String,
    },
    /// Generate shell integration script
    Activate {
        /// Shell type (bash, zsh, fish, nushell)
        shell: String,
    },
    /// Get profile for prompt display (fast, optimized for shell prompts)
    Prompt,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = Config::load()?;

    match cli.command {
        Commands::Add { name, user_name, email, signing_key } => {
            let profile = GitProfile {
                name: user_name,
                email,
                signing_key,
            };
            config.add_profile(name.clone(), profile);
            config.save()?;
            println!("Profile '{}' added successfully", name);
        }

        Commands::List => {
            if config.profiles.is_empty() {
                println!("No profiles configured");
                return Ok(());
            }

            println!("Available profiles:");
            for (name, profile) in &config.profiles {
                let current = if config.current_profile.as_ref() == Some(name) {
                    " (current)"
                } else {
                    ""
                };
                println!("  {} - {} <{}>{}", name, profile.name, profile.email, current);
                if let Some(key) = &profile.signing_key {
                    println!("    Signing key: {}", key);
                }
            }
        }

        Commands::Remove { name } => {
            if config.remove_profile(&name) {
                config.save()?;
                println!("Profile '{}' removed successfully", name);
            } else {
                println!("Profile '{}' not found", name);
            }
        }

        Commands::Switch { name } => {
            if let Some(profile) = config.get_profile(&name) {
                git::set_git_config(profile, true)?;
                config.set_current_profile(name.clone());
                config.save()?;
                println!("Switched to profile '{}' globally", name);
            } else {
                println!("Profile '{}' not found", name);
            }
        }

        Commands::Local { name } => {
            if !git::is_git_repo() {
                println!("Not in a git repository");
                return Ok(());
            }

            if let Some(profile) = config.get_profile(&name) {
                git::set_git_config(profile, false)?;
                println!("Switched to profile '{}' locally", name);
            } else {
                println!("Profile '{}' not found", name);
            }
        }

        Commands::Current { format } => {
            match git::get_current_git_config() {
                Ok(profile) => {
                    match format.as_str() {
                        "name" => println!("{}", profile.name),
                        "email" => println!("{}", profile.email),
                        "full" => {
                            println!("Current git configuration:");
                            println!("  Name: {}", profile.name);
                            println!("  Email: {}", profile.email);
                            if let Some(key) = profile.signing_key {
                                println!("  Signing key: {}", key);
                            }
                        }
                        _ => {
                            println!("Invalid format: {}. Valid formats: full, name, email", format);
                            return Ok(());
                        }
                    }
                }
                Err(e) => {
                    if format.as_str() == "full" {
                        println!("Failed to get current git configuration: {}", e);
                    }
                    // Silent for name/email format when there's an error
                }
            }
        }

        Commands::Auto => {
            if !git::is_git_repo() {
                println!("Auto-switching only works within git repositories");
                return Ok(());
            }

            if let Some(profile_name) = dotfile::get_dotfile_profile() {
                if let Some(profile) = config.get_profile(&profile_name) {
                    // Always apply locally since we're guaranteed to be in a git repo
                    git::set_git_config(profile, false)?;
                    println!("Auto-switched to profile '{}' locally", profile_name);
                } else {
                    println!("Profile '{}' specified in .gswitch file not found", profile_name);
                }
            } else {
                println!("No .gswitch file found in current git repository");
            }
        }

        Commands::Init { profile } => {
            if config.get_profile(&profile).is_none() {
                println!("Profile '{}' not found. Available profiles:", profile);
                for name in config.profiles.keys() {
                    println!("  {}", name);
                }
                return Ok(());
            }

            dotfile::create_dotfile(".gswitch", &profile)?;
            println!("Created .gswitch file with profile '{}'", profile);
        }

        Commands::Import { name } => {
            match git::get_current_git_config() {
                Ok(profile) => {
                    if config.profiles.contains_key(&name) {
                        println!("Profile '{}' already exists. Use a different name or remove the existing profile first.", name);
                        return Ok(());
                    }

                    config.add_profile(name.clone(), profile.clone());
                    config.save()?;
                    println!("Imported current git identity as profile '{}':", name);
                    println!("  Name: {}", profile.name);
                    println!("  Email: {}", profile.email);
                    if let Some(key) = profile.signing_key {
                        println!("  Signing key: {}", key);
                    }
                }
                Err(e) => {
                    println!("Failed to import current git configuration: {}", e);
                    println!("Make sure you have git configured with at least user.name and user.email");
                }
            }
        }

        Commands::Activate { shell } => {
            let script = match shell.as_str() {
                "bash" | "zsh" => {
                    r#"_gsw_auto_switch() {
    if command -v gsw >/dev/null 2>&1; then
        gsw auto 2>/dev/null
    fi
}

case "$-" in
    *i*) 
        if [[ "${shell}" == "zsh" ]]; then
            autoload -U add-zsh-hook
            add-zsh-hook chpwd _gsw_auto_switch
        else
            _gsw_original_cd=$(declare -f cd)
            cd() {
                builtin cd "$@" && _gsw_auto_switch
            }
        fi
        _gsw_auto_switch
        ;;
esac"#
                }
                "fish" => {
                    r#"function _gsw_auto_switch --on-variable PWD
    if command -v gsw >/dev/null 2>&1
        gsw auto 2>/dev/null
    end
end
_gsw_auto_switch"#
                }
                "nushell" => {
                    r#"def _gsw_auto_switch [] {
    if (which gsw | is-not-empty) {
        try { gsw auto } | ignore
    }
}

$env.config = ($env.config | upsert hooks {
    env_change: {
        PWD: [{ _gsw_auto_switch }]
    }
})

_gsw_auto_switch"#
                }
                _ => {
                    println!("Unsupported shell: {}. Supported shells: bash, zsh, fish, nushell", shell);
                    return Ok(());
                }
            };
            
            println!("{}", script);
        }

        Commands::Prompt => {
            // Fast path: only check current directory for .gswitch file
            // Use absolute path to ensure we're checking exactly the current directory
            let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let gswitch_path = current_dir.join(".gswitch");
            
            if gswitch_path.exists()
                && let Ok(content) = std::fs::read_to_string(&gswitch_path) {
                    let profile_name = content.trim();
                    if !profile_name.is_empty() && !profile_name.chars().all(|c| c.is_whitespace()) {
                        print!(" {}", profile_name);
                        std::process::exit(0);
                    }
                }
            // Exit with error code if no valid profile found
            // This tells Starship not to display anything
            std::process::exit(1);
        }
    }

    Ok(())
}
