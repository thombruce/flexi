# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Lines may start with `!` (focus) or `~` (obsolete, won't do) as well as `x`, at most one per line. Todos sort focused, open, done, then obsolete. Obsolete todos take a closed date like done ones. `--open` includes focused todos. Commands to set them come separately (#41); for now, add the marker with `toodles edit`.
- `list` tag queries now also match `#hashtag` tags (a `#` followed by a word starting with a letter, e.g. `#urgent`; `#42` isn't one). Quote it on the command line (`list '#urgent'`), or the shell treats it as a comment.

## [0.1.0] - 2026-10-18

### Added

- Initial version. A plaintext todo list, one todo per line: `[x ][(A) ][CLOSED ][CREATED ][DUE ]TITLE` — todo.txt's created/closed dates plus a third, positional due date (e.g. `(A) 2026-10-18 2026-10-31 Renew passport`), sorted by status, then priority, then due date, and hand-editable.
- `toodles add [--priority A-Z] [--due YYYY-MM-DD] <title...>` adds a todo, always stamping today as the created date.
- Bare `toodles` and `toodles list` show every todo, sorted; `--open`/`--done` and `+project`/`@context`/`key:value` tag queries filter the view without renumbering. Open todos past their due date are shown in red.
- `toodles done <n>` toggles done/open in place, stamping (or clearing) the closed date; priority is kept on completion.
- `add` refuses a title that starts with a date unless `--due` is given, since the date would otherwise be read back as the due date. Dates must be zero-padded `YYYY-MM-DD`; `2024-9-1` is plain title text.
- `toodles rm <n>`, `toodles edit`, `toodles completions <shell>`, `toodles man`.
