# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial release. A plaintext contacts / rolodex, one contact per line: `NAME [phone:VALUE]... [email:VALUE]...` (e.g. `John Smith phone:555-123-4567 email:john@example.com`), sorted alphabetically by name and hand-editable. `phone:`/`email:` tokens can appear anywhere in the line and are extracted (multiple allowed per contact); unreserved `+project`/`@context`/other `key:value` tags stay embedded in the name untouched, same convention as bagg/calchemy.
- `holler add <name...> [phone:N] [email:E] [+tag]...` adds a contact.
- Bare `holler` and `holler list` show every contact, sorted; optional `+project`/`@context`/`key:value` tag queries filter the view.
- `holler phone <query...>` / `holler email <query...>` look up a field by name substring (case-insensitive): a single match prints the bare value(s), pipe-friendly; multiple matches print `Name: value` per line to disambiguate; no match errors.
- `holler rm <n>` (alias `remove`) removes the contact at position `n` in `list`; `holler edit` opens the file in `$EDITOR`.
- `holler completions <shell>` and `holler man`.
- Config at `~/.config/holler/holler.toml`: `path`.
