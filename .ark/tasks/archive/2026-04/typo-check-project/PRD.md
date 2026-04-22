# `typo-check-project` PRD

---

[**What**]
Scan the entire project for typos in user-visible text (comments, docs, string literals, identifiers where safe) and fix them.

[**Why**]
Polish — typos erode trust in docs and make code harder to grep. No functional change, reversible in a single commit.

[**Outcome**]
- Every typo found in prose (Markdown docs, code comments, string literals) is either fixed or intentionally left (with reason noted below).
- Identifier typos are flagged but NOT auto-renamed unless clearly scoped and safe (renaming identifiers is out of scope for quick tier — promote if needed).
- `cargo check` (workspace) still passes after edits.
- Typos that are intentional (e.g., domain terms, third-party names, vendored code) are skipped and listed in the Verification notes below.

Verification checklist:
- [x] Scanned: `README.md`, `AGENTS.md`, all `*.md` under `docs/` (excluding `docs/archived/` — archive is historical record), `.ark/`, crate-level READMEs.
- [x] Scanned: Rust source comments and string literals under `xemu/`, `xam/`, `xlib/`, `xkernels/`, `scripts/` (skipped `target/`, vendored `resource/opensbi/`, vendored `xkernels/benchmarks/coremark/`, assembly files).
- [x] `cargo check` — N/A: the only fix is a C++ comment under `xkernels/benchmarks/microbench/` (kernel-side benchmark, not part of any Rust workspace). No Rust code changed. Git diff confirms a single-character change in one comment.
- [x] Diff reviewed; no semantic changes, only spelling.

Findings:
- `xkernels/benchmarks/microbench/src/ssort/ssort.cc:18` — `occurences` → `occurrences` (matches correct spelling already on line 15 of the same function).

Intentionally skipped:
- `resource/opensbi/**` — upstream OpenSBI vendor tree (`Recieve` in `uart8250.c`, `writeable` in domain docs). Not ours to touch.
- `xkernels/benchmarks/coremark/**` — upstream EEMBC CoreMark (`accomodate` x3). Not ours to touch.
- `docs/archived/**` — historical record per Ark's "archive is memory" principle (`truely` in `feat/trace/03_MASTER.md`). Preserved verbatim.

[**Related Specs**]
None — no feature specs exist yet.
