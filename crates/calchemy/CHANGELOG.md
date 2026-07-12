# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `calchemy add <date> HH:MM <end-date> HH:MM <title>` adds a timed appointment ending on a later day (previously the `YYYY-MM-DD HH:MM` end form was hand-edit only). The end must be after the start.

## [0.2.0] - 2026-07-12

### Added

- Multi-day all-day events: `DATE END-DATE # TITLE` (e.g. `2026-07-17 2026-07-20 # Wedding`, inclusive last day), and `calchemy add <date> <end-date> <title>`. The end date must be after the start date.

### Changed

- Appointments spanning several days (multi-day all-day, or timed events crossing midnight) now appear on every day they cover: an ongoing event shows in today's agenda and the default/windowed `list` views, and counts as `--past` only after its last day.

## [0.1.0] - 2026-07-11

### Added

- Initial release. A plaintext calendar of dated appointments, one per line: `DATE [START [END]] # TITLE` (e.g. `2026-07-14 09:00 10:00 # Dentist`), sorted chronologically on read and hand-editable.
- `calchemy add <date> [HH:MM[-HH:MM]] <title>` adds an appointment (explicit ISO dates; an end at or before the start rolls to the next day).
- Bare `calchemy` shows today's agenda; `calchemy next` shows the soonest upcoming appointment.
- `calchemy list` (alias `agenda`) shows upcoming appointments, with `--today`/`--week`/`--month`/`--since`/`--until` filters plus `--all` and `--past`; `--week`/`--month` cover the full calendar week/month. `today` and `week` shortcuts.
- `calchemy rm <n>` removes the nth appointment shown by `list`; `calchemy edit` opens the file in `$EDITOR`.
- `calchemy completions <shell>` and `calchemy man`.
- Config at `~/.config/calchemy/calchemy.toml`: `path`, `week_start`.
