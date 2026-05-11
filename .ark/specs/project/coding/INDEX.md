# Project Specs

Project-level conventions. User-authored.

A spec entry may be either:

- `<name>/SPEC.md`: a concrete spec for a specific scope.
- `<name>/INDEX.md`: a nested spec index that recursively points to more specs.

Use `INDEX.md` when a spec area contains multiple sub-specs and needs its own hierarchy.

## Index

| Spec              | Scope                 |
| ----------------- | --------------------- |
| `general/SPEC.md` | `General Guidelines`  |
| `asm/SPEC.md`     | `Assembly Guidelines` |
| `git/SPEC.md`     | `Git Guidelines`      |
| `testing/SPEC.md` | `Testing Guidelines`  |
| `rust/INDEX.md`   | `Rust Guidelines`     |

---

## How to Use

**When reading:** scan this table first. Open the relevant `SPEC.md`; if the entry is an `INDEX.md`, follow it recursively until you reach the concrete specs that apply to the files you will touch.

**When adding:** create either `<name>/SPEC.md` for a single focused spec, or `<name>/INDEX.md` when the area needs multiple child specs. Then append a row here or in the nearest parent `INDEX.md`.

**Ownership:** these files are user-authored and user-maintained. Agents may read them, but must never create, edit, or modify `SPEC.md` or `INDEX.md` files unless explicitly instructed by the user.
