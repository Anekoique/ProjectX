# Project Specs

Project-level conventions. **User-authored, user-maintained.** Agents may read these but must never create or modify SPEC / INDEX files unless explicitly instructed.

A spec entry is either:

- `<name>/SPEC.md` — a concrete spec for a specific scope.
- `<name>/INDEX.md` — a nested index that recursively points to more specs.

Use `INDEX.md` when an area has multiple sub-specs that need their own hierarchy.

## Index

| Spec                            | Scope                                                                    |
| ------------------------------- | ------------------------------------------------------------------------ |
| `<e.g. <language>/SPEC.md>`     | <e.g. language-specific style, error-handling, naming conventions>       |
| `<e.g. <area>/INDEX.md>`        | <e.g. an area with several child SPECs (architecture, security, …)>      |

<one row per spec or nested index. Keep the Scope column terse — agents scan this table to decide what to read.>

---

## How to use

- **Read:** scan the table; open the relevant `SPEC.md`. If the entry is an `INDEX.md`, follow it recursively until you reach the concrete specs that apply to the files you'll touch.
- **Add:** create either `<name>/SPEC.md` (single focused spec) or `<name>/INDEX.md` (area with multiple child specs). Append a row here, or in the nearest parent `INDEX.md`.
