# Feature Specs

Feature specifications extracted from deep-tier tasks at commit. Layout: `<subtree>/.../<feature>/SPEC.md`. The features tree is recursive; the rows below name immediate children — open a subtree's own `INDEX.md` to discover its leaves.

The table below is managed by `ark agent spec register` — rows appear when a deep-tier task is committed with a promoted SPEC. **Do not hand-edit rows between the markers.** Edit outside the block, or let the CLI do it.

## Index

<!-- ARK:FEATURES:START -->
| Feature              | Scope                                                                  | Promoted   |
| -------------------- | ---------------------------------------------------------------------- | ---------- |
| `xemu/INDEX.md`      | RISC-V emulator: ISA, CSRs, MMU, devices, difftest, perf.              | 2026-05-11 |
| `xlib/SPEC.md`       | Freestanding C library for xam-built guests (string / format / stdio). | 2026-05-11 |
| `port-to-ark/SPEC.md`| Port project workflow to Ark.                                          | 2026-05-11 |
<!-- ARK:FEATURES:END -->

---

## How to use

- **Read:** start with this root index, descend into the subtree(s) you'll touch via the linked `INDEX.md`s. Each subtree's `INDEX.md` lists its own leaves and any nested branches.
- **Modify a feature SPEC:** append a `[**CHANGELOG**]` entry inside the leaf. Ark re-writes the `Promoted` column at every parent INDEX up to root on the next deep commit that touches that SPEC.
