# clocking

A minimal CLI tool for logging your working hours — a traditional clock-in/clock-out timesheet backed by a plaintext, human-readable log.

Sibling to [`flexi`](../flexi/): where flexi tracks a running flexi-time *balance*, clocking records discrete worked *sessions* you total over a period.

## Quickstart

```sh
cargo install clocking
clocking in            # clock in
clocking               # clocked in since 09:00 (2 hr 30 min so far)
clocking out           # clocked out — worked 8 hr 30 min (09:00–17:30)
clocking summary --week  # Worked: 38 hr 15 min (5 sessions)
```

## Installation

**Homebrew** (macOS/Linux):
```sh
brew tap thombruce/tap
brew install thombruce/tap/clocking
```

**crates.io** (requires Rust):
```sh
cargo install clocking
```

**From source** (from the repo root):
```sh
cargo install --path crates/clocking
```

## Usage

```sh
clocking                   # status: clocked in, or today's total
clocking in                # clock in — start a work session
clocking in -m "project x" # clock in with an annotation
clocking out               # clock out — record the elapsed session
clocking out -m "reason"   # clock out with an annotation
clocking out --force       # record even if the session exceeds max_session
clocking log               # show worked sessions
clocking log --today       # today only (alias: --day)
clocking log --yesterday   # yesterday only
clocking log --week        # current calendar week
clocking log --month       # current calendar month
clocking log --since 2026-07-01 --until 2026-07-11  # date range
clocking log --last 10     # last 10 entries (combinable with filters)
clocking log --summary     # total worked (shortcut: `clocking summary`)
clocking summary --week    # total worked this week (takes the same filters)
clocking edit              # open log file in $EDITOR
clocking undo              # remove the last log entry
clocking completions <shell>  # print shell completion script
clocking man               # print the man page (roff) to stdout
```

`clocking in` opens a session; `clocking out` closes it and records the hours worked. Only one session is open at a time — a second `clocking in` is rejected until you clock out. While clocked in, bare `clocking` shows how long you've been clocked in; when clocked out it shows today's total. To discard an open session without recording it, run `clocking undo` while clocked in.

A closed session is logged as `<clock-in> <end> = <duration>`, keyed by the time you clocked **in**:

```
2026-07-11 09:00 17:00 = 8 hr           # a same-day shift
2026-07-11 20:00 2026-07-12 04:00 = 8 hr  # a shift crossing midnight
```

The leading timestamp is the clock-in time; the end field is a bare `HH:MM` for a same-day shift, or a full date + time when the shift crosses midnight (so no information is lost). The `= <duration>` is the recorded worked time — with `increment` rounding on, this is the rounded value and may differ from the raw span. Because sessions are dated by clock-in, a shift that runs past midnight counts on the day it *started*. (In `full` timestamp mode both endpoints are always written as full timestamps.)

`clocking summary` sums the `= <duration>` values over the selected period. `-m`/`--note` annotates the entry on either `in` or `out`; a note given at clock-in is kept and joined with any clock-out note.

clocking never asks you to type a duration — sessions are timed from the clock, so the only durations you see are computed output (shown in green as `N hr M min`).

### Shell completions

```sh
clocking completions zsh > ~/.zsh/completions/_clocking
clocking completions bash > ~/.bash_completion.d/clocking
clocking completions fish > ~/.config/fish/completions/clocking.fish
```

### Man page

```sh
clocking man > ~/.local/share/man/man1/clocking.1   # then: man clocking
```

## Configuration

Create `~/.config/clocking/clocking.toml` to configure clocking:

```toml
path = "/path/to/clocking.txt"
timestamp_format = "simple"  # "simple" (default) or "full"
week_start = "monday"        # "monday" (default) or "sunday"
increment = 1                # round session durations to this many minutes (default 1 = no rounding)
max_session = 1440           # warn/refuse if a session exceeds this many minutes (default: unset)
```

| `timestamp_format` | Example |
|--------------------|---------|
| `simple` (default) | `2026-07-11 17:30` |
| `full` | `2026-07-11T17:30:16+01:00` |

| `week_start` | Description |
|--------------|-------------|
| `monday` (default) | Week starts on Monday |
| `sunday` | Week starts on Sunday |

`increment` rounds each recorded session's elapsed time to the nearest multiple of that many minutes (half rounds up). `max_session` guards against a forgotten `clocking out`: a session longer than this many minutes refuses to record (run `clocking out --force` to record it anyway), and bare `clocking` warns while clocked in. Both must be at least 1; `max_session` is unset by default.

Without config, data is stored at `~/.local/share/clocking/clocking.txt` (or the platform equivalent). This file is an append-only log of worked sessions, each dated by the day you clocked in.

## License

MIT
