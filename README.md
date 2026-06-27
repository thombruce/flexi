# flexi

A minimal CLI tool for tracking your flexi-time balance.

## Quickstart

```sh
cargo install flexi
flexi set 3 hr # set your bank
flexi add 1 hr 30 min # add to your bank
flexi remove 1 hr # subtract from your bank
flexi log # show change history
flexi # show current balance (3 hr 30 min)
```

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
flexi add 1 hr --note "reason"  # add time with annotation
flexi remove 1 hr          # subtract time (alias: rm)
flexi in                   # start a clock-in session (bank time worked on `out`)
flexi out                  # stop the session and add the elapsed time
flexi set 2 hr             # set balance to exact value
flexi reset                # reset balance to zero
flexi reset --note "reason"  # reset with annotation
flexi note "annual leave"  # record a note without changing the balance
flexi log                  # show change history
flexi log --today          # today only (alias: --day)
flexi log --yesterday      # yesterday only
flexi log --week           # current calendar week
flexi log --month          # current calendar month
flexi log --since 2026-05-01 --until 2026-05-24  # date range
flexi log --last 10        # last 10 entries (combinable with filters)
flexi log --week --summary # totals for current week (added, removed, net)
flexi log --today --prose  # plain-sentence summary of the period and balance
flexi summary              # totals (shortcut for `log --summary`; takes the same filters)
flexi prose                # plain-sentence summary (shortcut for `log --prose`)
flexi log --json           # machine-readable entries (combine with --summary for totals)
flexi edit                 # open log file in $EDITOR
flexi undo                 # undo last change
flexi copy                 # copy balance to clipboard (alias: cp)
flexi completions <shell>  # print shell completion script
```

`add` and `remove` print the change and new balance (e.g. `+1 hr 30 min → 3 hr`). `set` and `reset` print the new balance. `note` records a dated, described entry with a `+0 min` change, leaving the balance untouched (useful for marking leave days, approvals, or reconciliation checkpoints); it is excluded from `--summary` and `--prose` totals. `log` prints one entry per line: `2026-05-24 10:20  +1 hr 30 min → 3 hr`. Notes appear dimmed at the end: `2026-05-24 10:20  +1 hr 30 min → 3 hr  # reason`.

`flexi in` starts a clock-in session and `flexi out` ends it, banking the elapsed time as overtime (it is *added* to your balance). While clocked in, bare `flexi` shows how long you've been clocked in, and balance-changing commands (`add`, `remove`, `set`, `reset`, `note`, a second `in`) are blocked until you run `flexi out`. The banked entry records the worked span as a note (e.g. `09:00–10:30`); pass `flexi in -m "reason"` or `flexi out -m "reason"` to annotate it. To cancel a session without banking anything, run `flexi undo` while clocked in.

`--note`/`-m` works on `add`, `remove`, `set`, `reset`, `in`, and `out`. Place it before or after the time args.

`flexi summary` and `flexi prose` are shortcuts for `flexi log --summary` and `flexi log --prose`. They accept the same date filters (`--today`, `--week`, `--since`, etc.) and default to the whole history when none is given.

`flexi log --json` emits machine-readable output (honours all date filters). Without `--summary` it prints an array of entries, each with `timestamp`, `delta_minutes` (`null` for `set`/`reset`), `balance_minutes`, and `note` (`null` if none). With `--summary` it prints a totals object `{ "added_minutes", "removed_minutes", "net_minutes" }`. All durations are in minutes. Pipe to `jq` for scripting. (`--json` conflicts with `--prose`.)

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

Create `~/.config/flexi/flexi.toml` to configure flexi:

```toml
path = "/path/to/flexi.txt"
timestamp_format = "simple"  # "simple" (default) or "full"
week_start = "monday"        # "monday" (default) or "sunday"
```

| `timestamp_format` | Example |
|--------------------|---------|
| `simple` (default) | `2026-05-24 10:20` |
| `full` | `2026-05-24T10:20:16+01:00` |

| `week_start` | Description |
|--------------|-------------|
| `monday` (default) | Week starts on Monday |
| `sunday` | Week starts on Sunday |

Without config, data is stored at `~/.local/share/flexi/flexi.txt` (or the platform equivalent). This file is the change log — the current balance is derived from the last entry.

## License

MIT
