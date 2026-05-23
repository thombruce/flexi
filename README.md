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

Time strings accept `hr`/`hrs`/`hour`/`hours` and `min`/`mins`/`minute`/`minutes`. Hours must come before minutes. Negative balances are displayed as e.g. `-1 hr 30 min`.

## Configuration

Create `~/.config/flexi/flexi.toml` to override the storage path:

```toml
path = "/path/to/flexi.txt"
```

Without config, the balance is stored at `~/.local/share/flexi/flexi.txt` (or the platform equivalent).

## License

MIT
