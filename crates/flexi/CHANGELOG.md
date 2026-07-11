# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `flexi start` and `flexi stop` — aliases for `in` and `out`. `in`/`out` remain the primary verbs; `start`/`stop` are an optional, arguably clearer reading for tracking overtime (clocking keeps `in`/`out` only, where they are semantically exact).

## [0.15.1] - 2026-07-11

### Changed

- Release tags are now per-crate: flexi releases from a `flexi-vX.Y.Z` tag (e.g. `flexi-v0.15.1`), not the old `vX.Y.Z`. The repository is a Cargo workspace and a single generic workflow releases each crate from its own `<crate>-vX.Y.Z` tag. Existing `vX.Y.Z` tags and their releases are unaffected.

## [0.15.0] - 2026-06-30

### Added

- `flexi away`/`flexi back` — a spending session that mirrors `in`/`out`: start it when you step out (long lunch, appointment), end it when you return, and the elapsed time is *subtracted* from your balance. Supports notes, span recording, `max_session`, `--force`, and `flexi undo`. `flexi out` and `flexi back` are interchangeable closers (only one session is open at a time; the sign is set when the session opens).

## [0.14.0] - 2026-06-27

### Added

- `flexi man` — prints a man page (roff) to stdout, e.g. `flexi man > ~/.local/share/man/man1/flexi.1`.

## [0.13.0] - 2026-06-27

### Added

- `flexi note "<text>"` — records a dated, described log entry with a `+0 min` change, leaving the balance untouched. Useful for marking leave days, approvals, or reconciliation checkpoints. Excluded from `--summary` and `--prose` totals.
- `flexi summary` and `flexi prose` — top-level shortcuts for `flexi log --summary` and `flexi log --prose`. Accept the same date filters and default to the whole history.
- `flexi log --json` — machine-readable output. Prints an array of entries (`timestamp`, `delta_minutes`, `balance_minutes`, `note`), or a totals object when combined with `--summary`. All durations in minutes. Honours date filters; conflicts with `--prose`.
- `flexi in` / `flexi out` — clock-in/out stopwatch. `in` opens a session (recorded as a balance-neutral `@in <balance>` marker); `out` banks the elapsed time as overtime, recording the worked span as a note. While clocked in, balance-changing commands are blocked, bare `flexi` shows elapsed time, and `flexi undo` cancels the session. Both accept `-m`/`--note`.
- `increment` config key — rounds every balance-changing duration (`add`, `remove`, `set`, and `in`/`out` elapsed) to the nearest multiple of N minutes (half rounds up). Defaults to `1` (exact minutes). Useful for workplaces that record flexi-time in fixed blocks, e.g. `increment = 15`.
- `max_session` config key + `flexi out --force` — safeguard against a forgotten clock-out. When an open session exceeds `max_session` minutes, `flexi out` refuses to bank it unless `--force` is given, and bare `flexi` shows a warning while clocked in. Unset by default (no limit).

### Changed

- `--note`/`-m` now rejects empty or whitespace-only values instead of writing a dangling `# ` suffix.

## [0.12.0] - 2026-06-23

### Added

- `flexi log --prose` — describes the period's change and current balance in a plain sentence (e.g. `Today: banked 1 hr 30 min (added 2 hr, removed 30 min). Balance now 1 hr 30 min.`). Works with any date filter (`--today`, `--week`, `--since`, etc.); conflicts with `--summary`.

## [0.11.1] - 2026-06-06

### Fixed
- Notes containing newlines are now rejected instead of silently corrupting the log file
- Zero-delta changes (e.g. `add 0 min`) no longer render in red

### Changed
- `add`/`remove`/`set`/`reset` now append to the log file in place instead of rewriting it on every change
- Reading the current balance now parses only the last log entry instead of the whole file

## [0.11.0] - 2026-05-30

### Added
- `--note`/`-m` flag on `reset` (consistent with `add`, `remove`, `set`)
- `flexi edit` — open the log file in `$EDITOR` (falls back to `vi`)

## [0.10.0] - 2026-05-30

### Added
- `flexi log --yesterday` — filter log entries to yesterday only
- `--note`/`-m` flag on `add`, `remove`, `set` — attach freeform annotation to a log entry (e.g. `flexi add 1 hr --note "stayed late"`); displayed dimmed in `flexi log`

## [0.9.0] - 2026-05-29

### Changed
- Log file now uses `>` as the separator between delta and new balance (e.g. `+1 hr > 2 hr`); `→` and `->` remain accepted for backwards compatibility and are displayed as `→` in the CLI
- `flexi log --summary` no longer shows current balance; totals reflect only the filtered period

## [0.8.0] - 2026-05-24

### Added
- `--version` flag
- `flexi log --summary` — shows totals (added, removed, net) for the filtered period; also shows current balance when no `--until` filter is set

## [0.7.0] - 2026-05-24

### Added
- `flexi log` filter flags: `--today`/`--day`, `--week`, `--month`, `--since YYYY-MM-DD`, `--until YYYY-MM-DD`, `--last N`
- `week_start` config key: `"monday"` (default) or `"sunday"`, controls `--week` filter
- `flexi log` entries colored: positive green, negative red, set/reset neutral
- `->` accepted as ASCII alias for `→` in hand-edited log entries; normalized to `→` in display

## [0.6.0] - 2026-05-24

### Added
- `timestamp_format` config key: `"simple"` (default, `2026-05-24 10:20`) or `"full"` (`2026-05-24T10:20:16+01:00`)
- `->` accepted as ASCII alias for `→` in hand-edited log entries

### Changed
- Storage simplified to a single file. `flexi.txt` is now the log — no separate `flexi.log`. Current balance is derived from the last entry's description.
- Log format changed from 4-column TSV (`timestamp\tprev\tnew\tdescription`) to 2-column (`timestamp description`). Timestamp and description are separated by whitespace and parsed by position.
- `reset` now records `= 0 min` in the log, consistent with `set`.

### Migration from 0.5.0

If you have an existing `flexi.log`, rename it to `flexi.txt` (replacing the old plain-text balance file). Then strip the integer columns:

```sh
awk -F'\t' '{print $1 " " $4}' flexi.txt > flexi.tmp && mv flexi.tmp flexi.txt
```

## [0.5.0] - 2026-05-23

### Added
- `flexi log` — show full change history
- `flexi undo` — revert the last change
- Delta output on `add` and `remove` (e.g. `+1 hr 30 min → 3 hr`)
- CI workflow for tests and clippy on push

## [0.4.0] - 2026-05-23

### Added
- `flexi copy` (alias: `cp`) — copy balance to clipboard; Wayland support via `wl-clipboard`
- Balance printed after every mutating command (`add`, `remove`, `set`, `reset`)

### Changed
- `rm` subcommand renamed to `remove`; `rm` kept as alias

## [0.3.0] - 2026-05-23

### Added
- `flexi set` — set balance to an exact value
- `flexi reset` — reset balance to zero
- `flexi completions` — print shell completion script
- Color output: positive balances green, negative red
- Atomic writes via `.tmp` file

## [0.2.0] - 2026-05-23

### Added
- Additional time input formats: plural/abbreviated units (`hours`, `hrs`, `mins`), compact (`1h30m`, `1h`, `30m`), decimal (`1.5`, `1.5h`), European decimal (`1,5`)

## [0.1.0] - 2026-05-23

### Added
- Initial CLI: `flexi` (display balance), `flexi add`, `flexi rm`
- XDG data directory support on Linux/macOS
- Negative balance display (e.g. `-1 hr 30 min`)
