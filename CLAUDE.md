# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workspace layout

Cargo workspace (`Cargo.toml` at root, `members = ["crates/*"]`). Each tool is its own crate under `crates/`:

- `crates/flexi` — track a running flexi-time balance.
- `crates/clocking` — log working hours as discrete clock-in/clock-out sessions.

Both share the same plaintext-log approach. `time.rs` (duration parse/format/round) is copied verbatim between them, and `config.rs` is copied with the crate name swapped. This duplication is **deliberate**: rule of three — a shared crate is extracted only once a third tool repeats the same code, not before. `storage.rs` is *not* shared: flexi keeps a running-balance chain (tail = current state via `new_minutes()`), while clocking records independent session durations summed over a period (`session_minutes()`), so the two storage layers diverge in semantics despite sharing the atomic-write/timestamp plumbing.

The rest of this file describes the `flexi` crate (`crates/flexi/`). The `clocking` crate mirrors flexi's module layout (`main.rs` + `config.rs` + `storage.rs` + `time.rs`, clap CLI, `@in` open-marker session model) with a leaner command set: `in`/`out`, `log`/`summary` (with the same date filters), `edit`, `undo`, `completions`, `man`. It has no balance, so no `add`/`remove`/`set`/`reset`, no clipboard, no json/prose. Its `out` pops the `@in` marker and appends `<duration> (HH:MM–HH:MM)`; `summary` sums those durations. Release wiring (workflow, homebrew, tag prefix) is flexi-only for now — see Releases.

## Commands

Cargo commands run from the repo root operate on the whole workspace; use `-p flexi` to scope to one crate.

```bash
cargo build          # compile all crates
cargo run -p flexi   # run flexi (display balance)
cargo run -p flexi -- add 1 hr 30 min
cargo run -p flexi -- add 1 hr 30 min --note "reason"
cargo run -p flexi -- remove 30 min
cargo test           # run all tests
cargo test time      # run tests in a specific module
cargo clippy         # lint (CI runs with -D warnings — fix all warnings before committing)
```

## Architecture

`flexi` is a single binary crate at `crates/flexi/` (`src/`, `tests/`). All state is `i32` minutes internally; `time.rs` owns the boundary between minutes and human-readable strings.

**Data flow for `add`/`remove`:**
`main.rs` joins `Vec<String>` args → `time::parse_duration` → arithmetic → `storage::append_log` → print delta.

**Modules:**
- `time.rs` — `parse_duration(s) -> i32`, `format_duration(i32) -> String`. All format rules live here. Negative balance renders as `-X hr Y min`.
- `config.rs` — reads `~/.config/flexi/flexi.toml` (optional `path` and `timestamp_format` keys). Falls back to `~/.local/share/flexi/flexi.txt`.
- `storage.rs` — `flexi.txt` is the log (single file). `append_log`, `read_log`, `pop_log`, `last_entry` all operate on this path. `read_minutes` derives current balance by parsing the last entry's description (`new_minutes()`). Log format: `timestamp description` — timestamp is fixed-width (16 chars simple, 25 chars full), parsed by position so any whitespace separator is accepted. Writes are atomic via `.tmp`. Notes are stored as ` # text` suffix in the description (e.g. `+30 min > 2 hr # stayed late`); both `delta_minutes()` and `new_minutes()` strip everything from ` # ` onward before parsing. Description forms: `+X > Y` / `-X > Y` (delta + new balance), `= Y` (set/reset), `@in Y` (open clock-in marker) and `@out Y` (open spending marker). Both markers are balance-neutral: `new_minutes()` reads `Y`, `delta_minutes()` returns `None`. `is_open_marker()` detects either; `is_spend()` distinguishes `@out`.
- `main.rs` — clap CLI plus the command handlers and the `run_log` rendering helper. Session state lives entirely in the log: `open_session()` returns the tail entry iff it is an `@in`/`@out` marker; `ensure_not_clocked_in()` guards balance mutations while a session is open. `flexi in`/`flexi away` open `@in`/`@out` markers; `flexi out` (alias `back`) pops the tail marker and appends a normal `+elapsed > balance` (work) or `-elapsed > balance` (spend) entry — sign chosen by `is_spend()`, not the closing verb (so the append-only running-balance chain is never rewritten mid-log). Only one session is open at a time, so either closing verb ends whichever is running.

**Time string format:** `N hr M min`, `N hr`, `M min`, `0 min`. Accepts plural/abbreviated unit words (`hour`, `hours`, `hrs`, `minute`, `minutes`, `mins`). Units are summed in any order (`30 min 1 hr` parses the same as `1 hr 30 min`); canonical output is always hours before minutes.

**Config file** (`~/.config/flexi/flexi.toml`):
```toml
path = "/custom/path/to/flexi.txt"
timestamp_format = "simple"   # "simple" (default): "2026-05-24 10:20"  |  "full": "2026-05-24T10:20:16+01:00"
week_start = "monday"         # "monday" (default) | "sunday"
increment = 1                 # round balance-changing durations to N minutes, nearest/half-up (default 1 = none); must be >= 1
max_session = 1440            # `out` refuses (without --force) if raw elapsed exceeds N minutes (default: unset/None); must be >= 1
```

Rounding (`time::round_to_increment`, nearest/half-up, sign-preserving) is applied in `main.rs` to every delta entering the balance — `add`/`remove`/`set` parsed input and `out` elapsed. `format_duration`/storage are unaffected; rounding happens before the value is written.

`max_session` (`Option<i32>`) is checked in the `out` handler against the **raw** elapsed (before rounding); over the cap it bails unless `flexi out --force` is passed. Bare `flexi` rounds the displayed "so far" elapsed and prints a warning line when over the cap.

## Documentation

Update `crates/flexi/README.md` whenever user-facing behaviour changes (new commands, flags, config keys, output format). The root `README.md` is a workspace overview — update it when crates are added or removed. Update `CLAUDE.md` when architecture or conventions change.

## Git workflow

Never commit directly to `main`. For every change, branch off `main`, commit there, push, and open a PR — even for small or solo work. Let CI (test + clippy) run before merging.

- Branch names: `type/short-description` (e.g. `feat/clocking-crate`, `chore/cargo-workspace`, `fix/rounding-sign`).
- Commit messages and PR titles follow Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`), matching the existing history.
- Keep the changelog current on the branch (see Releases) so PRs are self-contained.

## Releases

Each crate versions, changelogs, and releases independently. Add notable changes to the `[Unreleased]` section of that crate's `CHANGELOG.md` (e.g. `crates/clocking/CHANGELOG.md`) as they are made.

Before tagging a crate, move its `CHANGELOG.md` `[Unreleased]` section to a new version heading with today's date, bump the version in that crate's `Cargo.toml`, then build to update `Cargo.lock` (one lockfile at the workspace root). Commit all together.

**Tag scheme:** `<crate>-vX.Y.Z` (e.g. `flexi-v0.15.0`, `clocking-v0.1.0`). `.github/workflows/release.yml` is a single generic workflow triggered by any `*-vX.Y.Z` tag; its `setup` job derives the crate name and version from the tag (`crate = ${TAG%-v*}`, `version = ${TAG##*-v}`) and every downstream job is parameterised on them. To release: `git tag <crate>-vX.Y.Z && git push --tags`.

- **GitHub Release** — builds the four target archives (`<crate>-vX.Y.Z-<target>.tar.gz`) and attaches them.
- **crates.io** — `cargo publish -p <crate>` (requires the `CARGO_REGISTRY_TOKEN` secret). Manual fallback: same command locally after `cargo login`.
- **Homebrew tap** — writes `Formula/<crate>.rb` (class = capitalised crate, `desc` read from the crate's `Cargo.toml`, `test` runs `<crate> --version`) and pushes to `thombruce/homebrew-tap`.

Adding a new crate needs no workflow edits — it releases as soon as you push a `<crate>-vX.Y.Z` tag.
