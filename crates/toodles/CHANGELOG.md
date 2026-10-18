# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-10-18

### Added

- Lines may start with `!` (focus) or `~` (obsolete, won't do) as well as `x`, at most one per line. Todos sort focused, open, done, then obsolete. Obsolete todos take a closed date like done ones. `--open` includes focused todos.
- `toodles focus <n>` and `toodles wont <n>` toggle a todo's focus (`!`) or won't-do (`~`) status; running one again reopens it. `~` stamps the closed date, like `done`; moving between done and won't-do stamps it again, so it records when the todo got its current status. `list --wont` shows only won't-do todos.
- `list` tag queries now also match `#hashtag` tags (a `#` followed by a word starting with a letter, e.g. `#urgent`; `#42` isn't one). Quote it on the command line (`list '#urgent'`), or the shell treats it as a comment.

### Changed

- A line that already starts with a bare `!` or `~` word now reads it as a status marker rather than title text. For example, `~ 5 min stretch` becomes an obsolete todo titled `5 min stretch`, and any dates after it become closed dates. Write `!`/`~` attached to a word (`~5 min`) to keep it in the title.
- Dates may drop leading zeros in the month and day (`2030-1-5`), in the file and in `--due`, and are written back padded. The year must be 4 digits, so looser shapes like `1-2-3` are still plain title text.

## [0.1.0] - 2026-10-18

### Added

- Initial version. A plaintext todo list, one todo per line: `[x ][(A) ][CLOSED ][CREATED ][DUE ]TITLE` — todo.txt's created/closed dates plus a third, positional due date (e.g. `(A) 2026-10-18 2026-10-31 Renew passport`), sorted by status, then priority, then due date, and hand-editable.
- `toodles add [--priority A-Z] [--due YYYY-MM-DD] <title...>` adds a todo, always stamping today as the created date.
- Bare `toodles` and `toodles list` show every todo, sorted; `--open`/`--done` and `+project`/`@context`/`key:value` tag queries filter the view without renumbering. Open todos past their due date are shown in red.
- `toodles done <n>` toggles done/open in place, stamping (or clearing) the closed date; priority is kept on completion.
- `add` refuses a title that starts with a date unless `--due` is given, since the date would otherwise be read back as the due date. Dates must have a 4-digit year; the month and day may drop leading zeros (`2030-1-5`, also accepted by `--due`), and are written back padded. Looser shapes like `1-2-3` are plain title text.
- `toodles rm <n>`, `toodles edit`, `toodles completions <shell>`, `toodles man`.
