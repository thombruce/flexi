# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-10-18

### Added

- `list` tag queries now also match `#hashtag` tags (a `#` followed by a word starting with a letter, e.g. `#urgent`; `#42` isn't one). Quote it on the command line (`list '#urgent'`), or the shell treats it as a comment.

## [0.2.0] - 2026-10-18

### Changed

- **Breaking:** `edit` now runs `$EDITOR` through `sh`, as git does. It may now carry arguments (e.g. `code -w`), but an editor path containing spaces must be quoted inside `$EDITOR`; an unquoted one that used to work now fails. `edit` now needs `sh`, so it no longer works on Windows.
- `edit` reports an error when the editor can't be run (exit 126/127, e.g. a typo), and only warns for any other non-zero exit. It also creates the data directory first, so editing before the first entry works.

### Fixed

- Data-file rewrites are synced to disk before the atomic rename, so a crash can't leave the file empty or truncated.
- Adding an entry to a hand-edited file that lacks a final newline no longer glues the new line onto the last one.

## [0.1.0] - 2026-07-26

### Added

- Initial release. A plaintext contacts / rolodex, one contact per line: `NAME [phone:VALUE]... [email:VALUE]...` (e.g. `John Smith phone:555-123-4567 email:john@example.com`), sorted alphabetically by name and hand-editable. `phone:`/`email:` tokens can appear anywhere in the line and are extracted (multiple allowed per contact); unreserved `+project`/`@context`/other `key:value` tags stay embedded in the name untouched, same convention as bagg/calchemy.
- `holler add <name...> [phone:N] [email:E] [+tag]...` adds a contact.
- Bare `holler` and `holler list` show every contact, sorted; optional `+project`/`@context`/`key:value` tag queries filter the view.
- `holler phone <query...>` / `holler email <query...>` look up a field by name substring (case-insensitive): a single match prints the bare value(s), pipe-friendly; multiple matches print `Name: value` per line to disambiguate; no match errors.
- `holler rm <n>` (alias `remove`) removes the contact at position `n` in `list`; `holler edit` opens the file in `$EDITOR`.
- `holler completions <shell>` and `holler man`.
- Config at `~/.config/holler/holler.toml`: `path`.
