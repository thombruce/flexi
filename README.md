# flexi

A minimal CLI tool for tracking your flexi-time balance.

## Installation

**Homebrew** (macOS/Linux):
```sh
brew tap thombruce/tap
brew install thombruce/tap/flexi
```

**crates.io** (requires Rust):
```sh
cargo install flexi
```

**From source**:
```sh
cargo install --path .
```

## Usage

```sh
flexi                      # display current balance
flexi add 1 hr 30 min     # add time
flexi remove 1 hr          # subtract time (alias: rm)
flexi set 2 hr             # set balance to exact value
flexi reset                # reset balance to zero
flexi copy                 # copy balance to clipboard (alias: cp)
flexi completions <shell>  # print shell completion script
```

Mutation commands (`add`, `remove`, `set`, `reset`) print the updated balance after making changes.

Quotes are optional — `flexi add 1h30m` and `flexi add "1h30m"` are equivalent.

All of the following time formats are accepted:

| Format | Example |
|--------|---------|
| `N hr M min` | `1 hr 30 min`, `45 min`, `2 hr` |
| `N hour M minutes` | `1 hour 30 minutes`, `2 hours` |
| Compact | `1h30m`, `1h`, `30m`, `1.5h` |
| Decimal hours | `1.5`, `0.5` |
| Decimal hours with unit | `1.5 hours`, `1.5 hr` |
| European decimal | `1,5`, `1,5 hours` |

Positive balances display in green, negative in red. Negative balances display as e.g. `-1 hr 30 min`.

### Clipboard

`flexi copy` (or `flexi cp`) copies the current balance to the clipboard and prints it.

On **Wayland**, this requires the `wl-clipboard` package:

```sh
# Arch/Manjaro
pacman -S wl-clipboard
# Debian/Ubuntu
apt install wl-clipboard
# Fedora
dnf install wl-clipboard
```

On **macOS** and **X11 Linux**, no extra dependencies are needed.

### Shell completions

```sh
flexi completions zsh > ~/.zsh/completions/_flexi
flexi completions bash > ~/.bash_completion.d/flexi
flexi completions fish > ~/.config/fish/completions/flexi.fish
```

## Configuration

Create `~/.config/flexi/flexi.toml` to override the storage path:

```toml
path = "/path/to/flexi.txt"
```

Without config, the balance is stored at `~/.local/share/flexi/flexi.txt` (or the platform equivalent).

## License

MIT
