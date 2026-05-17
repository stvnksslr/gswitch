mod config;
mod dotfile;
mod git;

#[cfg(test)]
mod test_utils;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
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
        /// Signing key format: gpg, ssh, or x509 (optional)
        #[arg(long)]
        gpg_format: Option<String>,
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
        Commands::Add {
            name,
            user_name,
            email,
            signing_key,
            gpg_format,
        } => {
            let profile = GitProfile {
                name: user_name,
                email,
                signing_key,
                gpg_format,
            };
            config.add_profile(name.clone(), profile)?;
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
                println!(
                    "  {} - {} <{}>{}",
                    name, profile.name, profile.email, current
                );
                if let Some(key) = &profile.signing_key {
                    println!("    Signing key: {}", key);
                }
                if let Some(format) = &profile.gpg_format {
                    println!("    Signing format: {}", format);
                }
            }
        }

        Commands::Remove { name } => {
            if config.remove_profile(&name) {
                config.save()?;
                println!("Profile '{}' removed successfully", name);
            } else {
                bail!("Profile '{}' not found", name);
            }
        }

        Commands::Switch { name } => {
            if let Some(profile) = config.get_profile(&name) {
                git::set_git_config(profile, true)?;
                println!("Switched to profile '{}' globally", name);
                config.set_current_profile(name);
                config.save()?;
            } else {
                bail!("Profile '{}' not found", name);
            }
        }

        Commands::Local { name } => {
            if !git::is_git_repo() {
                bail!("Not in a git repository");
            }

            if let Some(profile) = config.get_profile(&name) {
                git::set_git_config(profile, false)?;
                println!("Switched to profile '{}' locally", name);
            } else {
                bail!("Profile '{}' not found", name);
            }
        }

        Commands::Current { format } => {
            let profile = git::get_current_git_config()?;
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
                    if let Some(format) = profile.gpg_format {
                        println!("  Signing format: {}", format);
                    }
                }
                _ => {
                    bail!(
                        "Invalid format: '{}'. Valid formats: full, name, email",
                        format
                    );
                }
            }
        }

        Commands::Auto => {
            // Early exit: Check for .gswitch file first (fastest check)
            let Some(profile_name) = dotfile::get_dotfile_profile() else {
                return Ok(()); // Silent exit when no .gswitch file - this is normal
            };

            // Early exit: Only proceed if in git repo
            if !git::is_git_repo() {
                return Ok(()); // Silent exit when not in git repo
            }

            // Check if we have the profile in config. Print to stdout (not
            // stderr): shell integration runs `gsw auto 2>/dev/null`, so a
            // stderr error would be swallowed and the user could commit under
            // the wrong identity with no warning at all.
            let Some(profile) = config.get_profile(&profile_name) else {
                println!(
                    "gswitch: profile '{}' from .gswitch is not configured; git identity left unchanged",
                    profile_name
                );
                return Ok(());
            };

            // Skip the write only if the *local* git identity already matches
            // the full profile. Reading the local scope (not the effective
            // config) ensures a matching global identity does not prevent the
            // per-repo config from being written; comparing the signing key
            // and gpg format avoids leaving a stale or missing key behind.
            let local_matches = git::get_local_config_value("user.name").as_deref()
                == Some(profile.name.as_str())
                && git::get_local_config_value("user.email").as_deref()
                    == Some(profile.email.as_str())
                && git::get_local_config_value("user.signingkey") == profile.signing_key
                && git::get_local_config_value("gpg.format") == profile.gpg_format;

            if local_matches {
                return Ok(()); // Already using correct profile, no need to switch
            }

            git::set_git_config(profile, false)?;
        }

        Commands::Init { profile } => {
            if config.get_profile(&profile).is_none() {
                let available: Vec<&str> = config.profiles.keys().map(|s| s.as_str()).collect();
                if available.is_empty() {
                    bail!(
                        "Profile '{}' not found. No profiles configured. Use 'gsw add' to create one.",
                        profile
                    );
                } else {
                    bail!(
                        "Profile '{}' not found. Available profiles: {}",
                        profile,
                        available.join(", ")
                    );
                }
            }

            dotfile::create_dotfile(".gswitch", &profile)?;
            println!("Created .gswitch file with profile '{}'", profile);
        }

        Commands::Import { name } => {
            if config.profiles.contains_key(&name) {
                bail!(
                    "Profile '{}' already exists. Use a different name or remove the existing profile first.",
                    name
                );
            }

            let profile = git::get_current_git_config()
                .map_err(|e| anyhow::anyhow!("Failed to import current git configuration: {}. Make sure you have git configured with at least user.name and user.email", e))?;

            config.add_profile(name.clone(), profile.clone())?;
            config.save()?;
            println!("Imported current git identity as profile '{}':", name);
            println!("  Name: {}", profile.name);
            println!("  Email: {}", profile.email);
            if let Some(ref key) = profile.signing_key {
                println!("  Signing key: {}", key);
            }
            if let Some(ref format) = profile.gpg_format {
                println!("  Signing format: {}", format);
            }
        }

        Commands::Activate { shell } => {
            let script = match shell.as_str() {
                "bash" => {
                    r#"_gsw_auto_switch() {
    if command -v gsw >/dev/null 2>&1; then
        gsw auto 2>/dev/null
    fi
}

case "$-" in
    *i*)
        cd() {
            builtin cd "$@"
            local _gsw_rc=$?
            _gsw_auto_switch
            return $_gsw_rc
        }
        _gsw_auto_switch
        ;;
esac"#
                }
                "zsh" => {
                    r#"_gsw_auto_switch() {
    if command -v gsw >/dev/null 2>&1; then
        gsw auto 2>/dev/null
    fi
}

case "$-" in
    *i*)
        autoload -U add-zsh-hook
        add-zsh-hook chpwd _gsw_auto_switch
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
                    bail!(
                        "Unsupported shell: '{}'. Supported shells: bash, zsh, fish, nushell",
                        shell
                    );
                }
            };

            println!("{}", script);
        }

        Commands::Prompt => {
            // Resolve the profile exactly the way `auto` does — searching up to
            // the repository root — so the prompt never disagrees with the
            // identity that auto-switching applied (e.g. from a subdirectory).
            if let Some(profile_name) = dotfile::get_dotfile_profile() {
                print!(" {}", profile_name);
                return Ok(());
            }
            // No profile: exit non-zero with no output so Starship (and other
            // prompts) display nothing — silent, with no error message.
            std::process::exit(1);
        }
    }

    Ok(())
}
