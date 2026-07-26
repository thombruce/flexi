# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-07-26

### Added

- Initial release. A plaintext shopping list / wishlist, one item per line: `[x ][(A) ][PRICE ]NAME[ xQTY]` (e.g. `x (A) 12.99 Widget thing x2`), sorted by got-status, then priority, then price, and hand-editable.
- `bagg add [--price 12.99] [--qty N] [--priority A-Z] <name...>` adds an item; quantity defaults to 1 and is omitted from the line when 1.
- Bare `bagg` and `bagg list` show every item, sorted; `--pending`/`--got` filter the view without renumbering.
- `bagg got <n>` toggles got/not-got status for the item at position `n` in `list`.
- `bagg rm <n>` (alias `remove`) removes the item at position `n` in `list`; `bagg edit` opens the file in `$EDITOR`.
- `bagg completions <shell>` and `bagg man`.
- Config at `~/.config/bagg/bagg.toml`: `path`.
