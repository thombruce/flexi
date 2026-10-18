# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial version. A plaintext todo list, one todo per line: `[x ][(A) ][CLOSED ][CREATED ][DUE ]TITLE` — todo.txt's created/closed dates plus a third, positional due date (e.g. `(A) 2026-10-18 2026-10-31 Renew passport`), sorted by status, then priority, then due date, and hand-editable.
- `toodles add [--priority A-Z] [--due YYYY-MM-DD] <title...>` adds a todo, always stamping today as the created date.
- Bare `toodles` and `toodles list` show every todo, sorted; `--open`/`--done` and `+project`/`@context`/`key:value` tag queries filter the view without renumbering. Open todos past their due date are shown in red.
- `toodles done <n>` toggles done/open in place, stamping (or clearing) the closed date; priority is kept on completion.
- `toodles rm <n>`, `toodles edit`, `toodles completions <shell>`, `toodles man`.
