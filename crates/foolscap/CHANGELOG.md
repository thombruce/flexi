# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `tags` module: `tags::matches(text, queries)` finds todo.txt-style `+project`/`@context`/`key:value` tags, plus `#hashtag` (`#` then a letter; a bare `#` is never a tag, so it can't clash with ` # note`), in free text (case-insensitive, matching if any query is present). It was previously copied into bagg, calchemy, holler and toodles. This is the first building block beyond plumbing (#53).
- `note` module: `note::split`, `note::append` and `note::parse` for an opt-in `<body> # <note>` suffix, starting at the first bare `#`. It was previously separate code in flexi and clocking.

## [0.1.0] - 2026-10-18

### Added

- Initial version: the plumbing shared by the flexi family of tools, extracted from six identical copies. XDG path discovery (`xdg_config_dir`, `xdg_data_dir`, `default_data_path`), toml config loading (`load_config::<T>`), line I/O (`read_lines`, `append_line`, and atomic `.tmp`+rename `write_lines`/`remove_line`/`replace_line`), and `open_in_editor`.
- Fixes for bugs that were present in every copy: `append_line` inserts a missing final newline before appending; `write_lines` names its temp file `<file>.tmp` instead of replacing the extension; and `open_in_editor` runs `$EDITOR` through `sh` (arguments and quoted paths work), fails if the editor can't be run (exit 126/127), and only warns on any other non-zero exit; it also creates the data directory first and requires `sh` (no Windows support). `write_lines` syncs the temp file to disk before renaming it.
