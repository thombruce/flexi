# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build          # compile
cargo run            # run (display balance)
cargo run -- add 1 hr 30 min
cargo run -- remove 30 min
cargo test           # run all tests
cargo test time      # run tests in a specific module
cargo clippy         # lint (CI runs with -D warnings — fix all warnings before committing)
```

## Architecture

Single binary crate. All state is `i32` minutes internally; `time.rs` owns the boundary between minutes and human-readable strings.

**Data flow for `add`/`remove`:**
`main.rs` joins `Vec<String>` args → `time::parse_duration` → arithmetic → `storage::append_log` → print delta.

**Modules:**
- `time.rs` — `parse_duration(s) -> i32`, `format_duration(i32) -> String`. All format rules live here. Negative balance renders as `-X hr Y min`.
- `config.rs` — reads `~/.config/flexi/flexi.toml` (optional `path` and `timestamp_format` keys). Falls back to `~/.local/share/flexi/flexi.txt`.
- `storage.rs` — `flexi.txt` is the log (single file). `append_log`, `read_log`, `pop_log` all operate on this path. `read_minutes` derives current balance by parsing the last entry's description (`new_minutes()`). Log format: `timestamp description` — timestamp is fixed-width (16 chars simple, 25 chars full), parsed by position so any whitespace separator is accepted. Writes are atomic via `.tmp`.
- `main.rs` — clap CLI only; no business logic.

**Time string format:** `N hr M min`, `N hr`, `M min`, `0 min`. Accepts plural/abbreviated unit words (`hour`, `hours`, `hrs`, `minute`, `minutes`, `mins`). Order must be hours before minutes.

**Config file** (`~/.config/flexi/flexi.toml`):
```toml
path = "/custom/path/to/flexi.txt"
timestamp_format = "simple"   # "simple" (default): "2026-05-24 10:20"  |  "full": "2026-05-24T10:20:16+01:00"
```

## Releases

**GitHub Releases** build automatically via `.github/workflows/release.yml` on `git tag vX.Y.Z && git push --tags`.

**crates.io:** `cargo publish` (requires `cargo login` first).

**Homebrew tap:** updated automatically on release via the Git workflow.
