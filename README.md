# flexi

A minimal CLI tool for tracking your flexi-time balance.

## Installation

**Homebrew** (macOS/Linux):
```sh
brew tap thombruce/tap
brew install flexi
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
flexi                   # display current balance
flexi add 1 hr 30 min  # add time
flexi add "45 min"      # quotes optional
flexi rm 1 hr           # subtract time
```

All of the following time formats are accepted:

| Format | Example |
|--------|---------|
| `N hr M min` | `1 hr 30 min`, `45 min`, `2 hr` |
| `N hour M minutes` | `1 hour 30 minutes`, `2 hours` |
| Compact | `1h30m`, `1h`, `30m`, `1.5h` |
| Decimal hours | `1.5`, `0.5` |
| Decimal hours with unit | `1.5 hours`, `1.5 hr` |
| European decimal | `1,5`, `1,5 hours` |

Negative balances display as e.g. `-1 hr 30 min`.

## Configuration

Create `~/.config/flexi/flexi.toml` to override the storage path:

```toml
path = "/path/to/flexi.txt"
```

Without config, the balance is stored at `~/.local/share/flexi/flexi.txt` (or the platform equivalent).

## License

MIT
