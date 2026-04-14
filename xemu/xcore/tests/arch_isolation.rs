//! Structural isolation test — locks in invariants I-1 and I-2 from
//! `docs/fix/archModule/03_PLAN.md`.
//!
//! Verifies that no source file outside `src/arch/` imports from
//! `crate::arch::riscv::` or `crate::arch::loongarch::` directly, except
//! for a small set of allow-listed seam files that re-export an enumerated
//! vocabulary of concrete arch types.
//!
//! The test walks the `xcore` crate source tree with `std::fs` only — no
//! dev-dep addition (plan constraint NG-7 / R-022).
//!
//! Limitation (R-019): this is a text-level check. It cannot distinguish
//! identifiers from strings, comments, or macro-expansion. Known
//! false-positive sources (debug-string literals `"aclint"` / `"plic"` in
//! `device/bus.rs`) are pinned per-occurrence in the allow-list below.

use std::{
    fs,
    path::{Path, PathBuf},
};

/// Relative path (from `xcore/`) of the source directory to walk.
const SRC_DIR: &str = "src";

/// Relative path of the `arch/` subtree — excluded from the non-seam scan.
const ARCH_DIR: &str = "src/arch";

/// Seam files: the ONLY files outside `src/arch/` permitted to reference
/// `crate::arch::riscv::` or `crate::arch::loongarch::`. Each entry is a
/// path-granular allow-list (not a whole-directory exception — R-014).
const SEAM_FILES: &[&str] = &[
    "src/arch/mod.rs",
    "src/cpu/mod.rs",
    "src/cpu/core.rs",
    "src/isa/mod.rs",
    "src/device/mod.rs",
    "src/device/intc/mod.rs",
];

/// Enumerated vocabulary: the ONLY symbol names seam files may re-export
/// from `crate::arch::<arch>::…` paths. Matches the `pub use` / `pub type`
/// aliases in the plan's API Surface section.
const SEAM_ALLOWED_SYMBOLS: &[&str] = &[
    // cpu/mod.rs seam aliases
    "Core",
    "CoreContext",
    "PendingTrap",
    "HartId",
    // device/intc/mod.rs seam re-exports
    "Aclint",
    "Plic",
    // device/mod.rs seam re-exports (mip bits)
    "SSIP",
    "MSIP",
    "STIP",
    "MTIP",
    "SEIP",
    "MEIP",
    "HW_IP_MASK",
    // isa/mod.rs seam re-exports
    "IMG",
    "DECODER",
    "DecodedInst",
    "InstFormat",
    "InstKind",
    "RVReg",
];

/// Pinned debug-string occurrences in `device/bus.rs`. Per the plan's
/// residual-risk note and R-024, the `add_mmio("aclint", …)` /
/// `add_mmio("plic", …)` sites survive as string literals and must be counted
/// so a broad string scan doesn't flag them. These are Bus-level NG-5 residuals
/// queued under `aclintSplit` / `plicGateway` / `directIrq`.
const BUS_DEBUG_STRING_PINS: &[(&str, usize)] = &[
    (r#""aclint""#, 0),
    (r#""plic""#, 1), // one `add_mmio("plic", …)` unit-test call site
];

/// Crate root (resolved from `CARGO_MANIFEST_DIR`).
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Collect every `.rs` file under `dir` (recursive).
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Return path relative to the crate root, using `/` as separator.
fn rel_to_crate_root(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Given a `use crate::arch::<arch>::...` line, extract every bare
/// identifier that appears at an import-leaf position, i.e. the names
/// after the final `::` (for a single-path use) or the names inside the
/// `{…}` brace group (for a multi-path use). Best-effort text parsing —
/// sufficient for the current seam vocabulary.
fn extract_leaf_idents(line: &str) -> Vec<String> {
    // Strip trailing `;` and whitespace.
    let line = line.trim().trim_end_matches(';').trim();
    // Drop any inline `//` comment tail.
    let line = match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    };

    if let (Some(open), Some(close)) = (line.find('{'), line.rfind('}')) {
        if open < close {
            let inner = &line[open + 1..close];
            return inner
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| {
                    // Handle `Name as Alias` — take the exported name.
                    let head = s.split_whitespace().next().unwrap_or("");
                    // Handle nested path like `trap::interrupt::MSIP` — take last.
                    head.rsplit("::").next().unwrap_or(head).to_string()
                })
                .collect();
        }
    }

    // Single-path use: take the final identifier after the last `::`.
    let tail = line.rsplit("::").next().unwrap_or("").trim();
    if tail.is_empty() {
        vec![]
    } else {
        // Handle `Name as Alias`.
        let head = tail.split_whitespace().next().unwrap_or("");
        vec![head.to_string()]
    }
}

/// True if `line` is a `pub use` / `use` / `pub type` statement that
/// references an arch path.
fn is_arch_use_line(line: &str) -> bool {
    let stripped = line.trim_start();
    (stripped.starts_with("use ") || stripped.starts_with("pub use "))
        && (stripped.contains("crate::arch::riscv::")
            || stripped.contains("crate::arch::loongarch::"))
}

/// Count non-overlapping occurrences of `needle` in `haystack`.
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut pos = 0;
    while let Some(off) = haystack[pos..].find(needle) {
        count += 1;
        pos += off + needle.len();
    }
    count
}

#[test]
fn arch_isolation() {
    let root = crate_root();
    let src = root.join(SRC_DIR);
    let arch = root.join(ARCH_DIR);

    assert!(
        src.is_dir(),
        "arch_isolation: {} does not exist",
        src.display()
    );

    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    assert!(
        !files.is_empty(),
        "arch_isolation: no .rs files found under {}",
        src.display()
    );

    let mut violations: Vec<String> = Vec::new();

    for path in &files {
        // Skip files under `src/arch/` — the arch backends own those paths.
        if path.starts_with(&arch) {
            continue;
        }

        let rel = rel_to_crate_root(path, &root);
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                violations.push(format!("{rel}: failed to read ({e})"));
                continue;
            }
        };

        let is_seam = SEAM_FILES.contains(&rel.as_str());

        for (lineno, raw) in content.lines().enumerate() {
            let lineno = lineno + 1;
            if !raw.contains("crate::arch::riscv::") && !raw.contains("crate::arch::loongarch::") {
                continue;
            }
            // Ignore pure doc / inner comments (lines whose first non-ws
            // token is `//`, `///`, or `//!`).
            let trimmed = raw.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }

            if !is_seam {
                // I-1 violation: non-seam file outside arch/ references an
                // arch path.
                violations.push(format!(
                    "{rel}:{lineno}: non-seam file references crate::arch::<arch>::  — line: `{}`",
                    raw.trim()
                ));
                continue;
            }

            // Seam file: must be a `use` / `pub use` / `pub type`
            // statement whose leaf idents are all in the allow-list.
            let is_type_alias = trimmed.starts_with("pub type ") || trimmed.starts_with("type ");
            let is_use = is_arch_use_line(raw);

            if !is_type_alias && !is_use {
                violations.push(format!(
                    "{rel}:{lineno}: seam file contains non-alias arch reference — line: `{}`",
                    raw.trim()
                ));
                continue;
            }

            // For `pub type Name = crate::arch::…::Concrete;`, the
            // allow-listed name is the one on the LHS (Name), which
            // must be in SEAM_ALLOWED_SYMBOLS.
            if is_type_alias {
                // Extract the LHS name: `pub type <Name> = …`
                let after_type = trimmed
                    .trim_start_matches("pub type ")
                    .trim_start_matches("type ");
                let name = after_type
                    .split(|c: char| c == '=' || c.is_whitespace() || c == '<')
                    .find(|s| !s.is_empty())
                    .unwrap_or("");
                if !SEAM_ALLOWED_SYMBOLS.contains(&name) {
                    violations.push(format!(
                        "{rel}:{lineno}: seam type alias `{name}` not in allow-list — line: `{}`",
                        raw.trim()
                    ));
                }
                continue;
            }

            // Multi-symbol `use …::{A, B, C};` or single `use …::X;`.
            let idents = extract_leaf_idents(raw);
            if idents.is_empty() {
                violations.push(format!(
                    "{rel}:{lineno}: seam use line parsed to zero idents — line: `{}`",
                    raw.trim()
                ));
                continue;
            }
            for id in idents {
                if !SEAM_ALLOWED_SYMBOLS.contains(&id.as_str()) {
                    violations.push(format!(
                        "{rel}:{lineno}: seam re-exports `{id}` which is not in allow-list — \
                         line: `{}`",
                        raw.trim()
                    ));
                }
            }
        }
    }

    // Pin known debug-string occurrences in device/bus.rs (NG-5).
    let bus_path = root.join("src/device/bus.rs");
    if let Ok(bus_src) = fs::read_to_string(&bus_path) {
        for (needle, expected) in BUS_DEBUG_STRING_PINS {
            let actual = count_occurrences(&bus_src, needle);
            if actual != *expected {
                violations.push(format!(
                    "src/device/bus.rs: debug-string {needle} occurrence count changed: expected \
                     {expected}, found {actual} (update BUS_DEBUG_STRING_PINS or relocate the \
                     site into `arch/riscv/`)",
                ));
            }
        }
    } else {
        violations.push("src/device/bus.rs: failed to read for debug-string pin check".into());
    }

    assert!(
        violations.is_empty(),
        "arch_isolation invariants violated ({} issues):\n  - {}",
        violations.len(),
        violations.join("\n  - ")
    );
}
