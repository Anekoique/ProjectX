"""Tiny Rust v0 / Itanium mangled-name cleaner.

Not a full demangler — we only need to recover a readable `crate::mod::leaf`
tail for chart labels.  Hash tokens, stutter, and v0 padding segments are
dropped.
"""
from __future__ import annotations

import re

KEEP_CRATES = {"xcore", "xdb", "xlogger", "xkernel"}
DROP_SEGMENTS = {"NtB", "NtBe", "NtB2", "NtB4", "NtB6", "NtB7", "NtBc"}


def _v0_segments(sym: str) -> list[str]:
    out: list[str] = []
    i = 0
    while i < len(sym):
        j = i
        while j < len(sym) and sym[j].isdigit():
            j += 1
        if j == i:
            i += 1
            continue
        try:
            n = int(sym[i:j])
        except ValueError:
            i = j
            continue
        if n <= 0 or n > 60 or j + n > len(sym):
            i = j
            continue
        seg = sym[j : j + n]
        if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", seg):
            out.append(seg)
            i = j + n
        else:
            i = j
    return out


def _keep(tok: str) -> bool:
    if tok in DROP_SEGMENTS:
        return False
    if re.fullmatch(r"Cs[A-Za-z0-9_]+", tok):
        return False
    if re.fullmatch(r"_?\d+[A-Za-z_][A-Za-z0-9_]*", tok):
        return False
    if re.fullmatch(r"s\d+_", tok):
        return False
    if re.fullmatch(r"_\d+", tok):
        return False
    return True


def clean(sym: str, max_len: int = 80) -> str:
    """Reduce a mangled symbol to a human-readable tail."""
    sym = sym.strip()
    if sym.startswith("_R"):
        toks = [t for t in _v0_segments(sym) if _keep(t)]
        if not toks:
            return sym[:max_len]
        anchor = next((i for i, t in enumerate(toks) if t in KEEP_CRATES), 0)
        tail = toks[anchor:]
        cleaned: list[str] = []
        for t in tail:
            if cleaned and cleaned[-1] == t:
                continue
            cleaned.append(t)
        return "::".join(cleaned)[:max_len]
    return sym[:max_len]
