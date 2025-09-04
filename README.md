# gswitch

A fast, lightweight CLI tool for managing and automatically switching git profiles based on project context.

## Features

- **Profile Management**: Store multiple git identities (name, email, signing key)
- **Smart Switching**: Automatically switch profiles when entering git repositories
- **Project-Specific**: Uses `.gswitch` dotfiles for per-project profile configuration
- **Git-Aware**: Only operates within git repositories, respects repository boundaries
- **Local by Default**: Auto-switching applies profiles locally to the current repository

## Installation

1. Clone this repository
2. Build with Cargo:
   ```bash
   cargo build --release
   ```
3. The binary will be available as `gsw` in `target/release/`
4. Copy to a directory in your PATH, e.g.:
   ```bash
   cp target/release/gsw ~/.local/bin/
   # or
   cp target/release/gsw /usr/local/bin/
   ```

## Quick Start

1. **Add a profile**:
   ```bash
   gsw add work --user-name "John Doe" --email "john@company.com"
   gsw add personal --user-name "John Doe" --email "john@personal.com"
   ```

2. **Import your current git identity**:
   ```bash
   gsw import current
   ```

3. **List profiles**:
   ```bash
   gsw list
   ```

4. **Switch profiles**:
   ```bash
   gsw switch work        # Switch globally
   gsw local personal     # Switch locally (current repo only)
   ```

5. **Set up project-specific switching**:
   ```bash
   cd /path/to/work/project
   gsw init work          # Creates .gswitch file
   gsw auto              # Auto-switches to work profile
   ```

## Commands

| Command | Description |
|---------|-------------|
| `gsw add <name> --user-name "Name" --email "email@example.com" [--signing-key "key"]` | Add a new profile |
| `gsw import <name>` | Import current git identity as a profile |
| `gsw list` | List all profiles |
| `gsw switch <name>` | Switch to profile globally |
| `gsw local <name>` | Switch to profile locally (current repo) |
| `gsw current` | Show current git configuration |
| `gsw init <name>` | Create .gswitch file in current directory |
| `gsw auto` | Auto-switch based on .gswitch file |
| `gsw activate <shell>` | Generate shell integration script |
| `gsw prompt` | Get profile for prompt display (optimized for speed) |
| `gsw remove <name>` | Remove a profile |

## Shell Integration

Enable automatic profile switching when changing directories with just one line:

### Bash
```bash
echo 'eval "$(gsw activate bash)"' >> ~/.bashrc
```

### Zsh  
```zsh
echo 'eval "$(gsw activate zsh)"' >> ~/.zshrc
```

### Fish
```fish
echo 'gsw activate fish | source' >> ~/.config/fish/config.fish
```

### Nushell
```nu
echo 'gsw activate nushell | source' >> ~/.config/nushell/config.nu
```

**Note**: Restart your shell or run `source ~/.bashrc` (or equivalent) for the integration to take effect.

## Starship Integration

Display the active git profile in your [Starship](https://starship.rs/) prompt by adding this to your `~/.config/starship.toml`:

### Fast option (recommended)

```toml
[custom.gsw]
command = "gsw prompt"
detect_files = [".gswitch"]
format = "[$output]($style)"
style = "bold blue"
description = "Show project git profile"
```

### Alternative: Show current git identity name

```toml
[custom.gsw]
command = "gsw current --format=name 2>/dev/null || echo ''"
detect_files = [".gswitch"]
format = "[$output]($style)"
style = "bold blue"
description = "Show active git identity"


This will append the git profile to the end of your prompt: `gswitch on master personal` when you're in a git repository with a `.gswitch` file.

## How It Works

1. **Directory Walking**: When you run `gsw auto`, it searches from your current directory up to the git repository root for a `.gswitch` file
2. **Repository Boundaries**: The search is constrained to the current git repository - it won't traverse outside
3. **Local Application**: Found profiles are applied locally to the current repository using `git config --local`
4. **Shell Integration**: The shell hooks automatically run `gsw auto` when you change directories

## Configuration

- Profiles are stored in `~/.config/gswitch/config.toml`
- Each project can have a `.gswitch` file containing the profile name to use
- The tool respects git repository boundaries and only operates within git repos

## Examples

```bash
# Set up work profile
gsw add work --user-name "Jane Smith" --email "jane@company.com" --signing-key "ABC123"

# Set up personal profile  
gsw add personal --user-name "Jane Smith" --email "jane@gmail.com"

# In a work project
cd ~/work/project
gsw init work
# Now every time you cd into this project, it will use the work profile

# In a personal project
cd ~/personal/project  
gsw init personal
# This project will automatically use the personal profile
```