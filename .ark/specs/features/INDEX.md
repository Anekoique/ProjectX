# Feature Specs

Feature specifications extracted from deep-tier tasks at commit. Layout: `<feature>/SPEC.md`.

The table below is managed by `ark agent spec register` — new rows appear when a deep-tier task is committed with a promoted SPEC. **Do not hand-edit rows between the markers.** Edit outside the block, or let the CLI do it.

## Index

<!-- ARK:FEATURES:START -->
| Feature              | Scope                                                                  | Promoted   |
| -------------------- | ---------------------------------------------------------------------- | ---------- |
| `xemu/INDEX.md`      | RISC-V emulator: ISA, CSRs, MMU, devices, difftest, perf.              | 2026-05-11 |
| `xlib/SPEC.md`       | Freestanding C library for xam-built guests (string / format / stdio). | 2026-05-11 |
| `port-to-ark/SPEC.md`| Port project workflow to Ark.                                          | 2026-05-11 |
| `xvisor/INDEX.md` | add xvisor trap | 2026-05-20 from task `trap` |

<!-- ARK:FEATURES:END -->

---

## How to use

- **Read:** scan the table; open the SPEC for any feature you'll touch.
- **Modify a feature SPEC:** append a `[**CHANGELOG**]` entry. Ark re-writes the `Promoted` column with the latest touch date.
