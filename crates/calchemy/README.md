# calchemy

A minimal plaintext calendar and appointment book for the command line — todo.txt for your diary.

Part of the [flexi](../../) family of plaintext-storage time tools; where flexi tracks a flexi-time balance and clocking logs worked hours, calchemy keeps a sortable, hand-editable list of dated appointments.

## Quickstart

```sh
cargo install calchemy
calchemy add 2026-07-14 09:00-10:00 Dentist @clinic
calchemy add 2026-12-25 Christmas
calchemy              # today's agenda
calchemy next         # the soonest upcoming appointment
calchemy list         # all upcoming
```

## Installation

**Homebrew** (macOS/Linux):
```sh
brew tap thombruce/tap
brew install thombruce/tap/calchemy
```

**crates.io** (requires Rust):
```sh
cargo install calchemy
```

**From source** (from the repo root):
```sh
cargo install --path crates/calchemy
```

## Usage

```sh
calchemy                              # today's agenda
calchemy add 2026-07-14 Dentist       # all-day appointment
calchemy add 2026-07-14 09:00 Standup # timed
calchemy add 2026-07-14 09:00-10:00 Dentist @clinic  # with an end time
calchemy add 2026-07-17 2026-07-20 Wedding  # multi-day all-day
calchemy add 2026-07-17 09:00 2026-07-20 17:00 Conference  # timed, ends on a later day
calchemy next                         # the soonest upcoming appointment
calchemy list                         # upcoming appointments (alias: agenda)
calchemy list --today                 # today only
calchemy list --week                  # the current calendar week (full week)
calchemy list --month                 # the current calendar month (full month)
calchemy list --since 2026-07-01 --until 2026-07-31  # a date range
calchemy list --all                   # include past appointments
calchemy list --past                  # only past appointments
calchemy today                        # shortcut for `list --today`
calchemy week                         # shortcut for `list --week`
calchemy rm 2                         # remove appointment #2 as shown by `list`
calchemy edit                         # open the calendar file in $EDITOR
calchemy completions <shell>          # print a shell completion script
calchemy man                          # print the man page (roff) to stdout
```

`add` takes an explicit `YYYY-MM-DD` date, then optionally a time (`HH:MM`), a time range (`HH:MM-HH:MM`), a timed end on a later day (`HH:MM YYYY-MM-DD HH:MM`), or an end date (`YYYY-MM-DD`, making a multi-day all-day event through that day inclusive), then the title. If the end of a time range is at or before the start, it is taken to be the next day (`20:00-02:00` → ends 02:00 tomorrow). A leading title word that looks like a time or date is claimed by these forms — an end date after a start time is only claimed when followed by an end time.

An appointment spanning several days appears on every day it covers — a multi-day event that started yesterday still shows in today's agenda and the upcoming `list`, and only moves to `--past` once its last day has passed.

Appointments are always listed in chronological order regardless of their order in the file; all-day events sort before timed ones on the same day. `list` shows upcoming appointments (today onward) by default — use `--all`, `--past`, or a date filter to see others. Unlike flexi/clocking's backward-looking logs, `--week` and `--month` show the **whole** calendar week/month, including future days.

`rm <n>` removes the appointment at position `n` in the default `list` view (upcoming, chronological). For anything more involved, `calchemy edit` opens the raw file.

## Storage format

The calendar is a plaintext file, one appointment per line, hand-editable:

```
2026-12-25 # Christmas
2026-07-14 09:00 # Standup
2026-07-14 09:00 10:00 # Dentist @clinic
2026-07-14 20:00 2026-07-15 02:00 # Party
2026-07-17 2026-07-20 # Wedding
```

Each line is `DATE [START [END]] # TITLE`:

- `DATE` is `YYYY-MM-DD` and always leads the line, so the file greps and sorts chronologically.
- `START` is `HH:MM`; omit it for an all-day event.
- `END` is `HH:MM` for a same-day finish, or `YYYY-MM-DD HH:MM` when the appointment runs into a later day. With no `START`, a bare `YYYY-MM-DD` end makes a multi-day all-day event (inclusive last day; must be after `DATE`).
- Everything after ` # ` is the title (free text; `@location` and `+tag` conventions can live inside it).

The ` # ` separator matches the note convention in flexi and clocking — machine-readable fields before it, human text after.

## Configuration

Create `~/.config/calchemy/calchemy.toml`:

```toml
path = "/path/to/calchemy.txt"
week_start = "monday"   # "monday" (default) or "sunday"
```

Without config, the calendar lives at `~/.local/share/calchemy/calchemy.txt` (or the platform equivalent).

## Not yet supported

Recurring appointments, natural-language dates (`next friday`), and JSON output are deliberately left out of this first version.

## License

MIT
