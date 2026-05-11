# Project Specs

Project-level conventions. User-authored.

A spec entry may be:

- `<name>.md`: a concrete spec for a specific scope.

## Index

| Spec                                       | Scope                                                |
| ------------------------------------------ | ---------------------------------------------------- |
| `comments-and-documentation.md`            | Rustdoc style, regular comments, what to document    |
| `concurrency-and-races.md`                 | Lock ordering, atomics, memory ordering, races       |
| `defensive-programming.md`                 | Assertions and invariant checks                      |
| `error-handling.md`                        | `Result`, `?`, typed errors, no `.unwrap()` policy   |
| `functions-and-methods.md`                 | Function shape, nesting, signatures                  |
| `logging.md`                               | Log levels, message style, structured fields         |
| `macros-and-attributes.md`                 | Attribute order, derives, macro discipline           |
| `memory-and-resource-management.md`        | Ownership, lifetimes, RAII for resources             |
| `modules-and-crates.md`                    | Module visibility, crate boundaries, re-exports      |
| `naming.md`                                | Naming guidelines                                    |
| `performance.md`                           | Hot-path discipline, allocations, benchmarking       |
| `types-and-traits.md`                      | Type-driven invariants, trait design                 |
| `unsafety.md`                              | `// SAFETY:` requirement, `unsafe` budget rules      |
| `variables-expressions-and-statements.md`  | Variable binding, expression clarity, immutability   |

---

## How to Use

**When reading:** scan this table first. Open the relevant `<name>.md`.

**When adding:** create `<name>.md` for a single focused spec.

**Ownership:** these files are user-authored and user-maintained. Agents read them, but must never create, edit, or modify `<name>.md`.
