# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-07-11

### Changed

- **Log format (breaking).** A closed session is now written as `<clock-in> <end> = <duration>` (e.g. `2026-07-11 09:00 17:00 = 8 hr`), keyed by clock-in time, instead of `<clock-out> <duration> (<span>)`. This removes the duplicated clock-out time and dates each session by when it *started*, so a shift that crosses midnight counts on its start day. When a session spans midnight the end field carries the full end date (`2026-07-11 20:00 2026-07-12 04:00 = 8 hr`); in `full` timestamp mode both endpoints are always full timestamps. Logs written by 0.1.0 are not read by 0.2.0 — sessions no longer appear in summaries. Given 0.1.0 released the same day, migration tooling is not provided; hand-edit any existing log to the new form.

## [0.1.0] - 2026-07-11

### Added

- Initial release. `clocking in`/`clocking out` open and close a work session, recording the hours worked as `8 hr 30 min (09:00–17:30)` in a plaintext log.
- Bare `clocking` shows the open session's elapsed time, or the day's total when clocked out.
- `clocking log` and `clocking summary` with `--today`/`--yesterday`/`--week`/`--month`/`--since`/`--until`/`--last` filters; `summary` totals the worked sessions.
- `-m`/`--note` annotations on `in` and `out`; notes from both are kept.
- `clocking edit`, `clocking undo`, `clocking completions <shell>`, `clocking man`.
- Config at `~/.config/clocking/clocking.toml`: `path`, `timestamp_format`, `week_start`, `increment`, `max_session`.
