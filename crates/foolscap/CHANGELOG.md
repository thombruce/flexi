# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial version: the plumbing shared by the flexi family of tools, extracted from six identical copies. XDG path discovery (`xdg_config_dir`, `xdg_data_dir`, `default_data_path`), toml config loading (`load_config::<T>`), line I/O (`read_lines`, `append_line`, and atomic `.tmp`+rename `write_lines`/`remove_line`/`replace_line`), and `open_in_editor`.
- Fixes for bugs that were present in every copy: `append_line` inserts a missing final newline before appending; `write_lines` names its temp file `<file>.tmp` instead of replacing the extension; and `open_in_editor` runs `$EDITOR` through `sh` (arguments and quoted paths work) and only warns on a non-zero exit.
