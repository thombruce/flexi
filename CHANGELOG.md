# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
