#!/usr/bin/env python3
"""scripts/perf/render.py — render every SVG chart for one perf run.

Reads a dated perf directory's ``data/`` subfolder and writes every SVG
into its ``graphics/`` subfolder.  Zero third-party deps (stdlib only).

Usage
-----
    python3 scripts/perf/render.py [--dir DIR]

``--dir`` defaults to the newest ``docs/perf/<YYYY-MM-DD>/`` under the
project root; pass an explicit path to re-render an older run.
"""
from __future__ import annotations

import argparse
import csv
import math
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from lib._demangle import clean  # type: ignore  # noqa: E402

PROJECT_ROOT = HERE.parent.parent

PALETTE = [
    "#2563eb", "#059669", "#dc2626", "#d97706", "#7c3aed",
    "#0891b2", "#db2777", "#4b5563", "#115e59", "#b45309",
]


# --- SVG primitives --------------------------------------------------
def _svg(w: int, h: int, title: str) -> list[str]:
    return [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" '
        f'width="{w}" height="{h}" font-family="ui-sans-serif,Helvetica" font-size="12">',
        f'<rect width="{w}" height="{h}" fill="#ffffff"/>',
        f'<text x="{w/2}" y="22" text-anchor="middle" font-weight="bold">{title}</text>',
    ]


def _text(x, y, s, **kw):
    a = " ".join(f'{k.replace("_","-")}="{v}"' for k, v in kw.items())
    return f'<text x="{x}" y="{y}" {a}>{s}</text>'


def _rect(x, y, w, h, fill, **kw):
    a = " ".join(f'{k.replace("_","-")}="{v}"' for k, v in kw.items())
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}" {a}/>'


def _write(path: Path, lines: list[str]) -> None:
    lines.append("</svg>")
    path.write_text("\n".join(lines))
    print(f"wrote {path.relative_to(PROJECT_ROOT)}")


# --- Bench bars from bench.csv ---------------------------------------
def _load_bench(data: Path) -> dict[str, tuple[float, float, float, int]]:
    best: dict[str, tuple[float, float, float, int]] = {}
    csv_path = data / "bench.csv"
    if not csv_path.exists():
        return best
    with csv_path.open() as f:
        for r in csv.DictReader(f):
            w = r["workload"]
            real = float(r["real_s"])
            if w not in best or real < best[w][0]:
                best[w] = (real, float(r["user_s"]), float(r["sys_s"]), int(r["max_rss_kb"]))
    return best


def _render_time(best, out: Path) -> None:
    labels = list(best)
    if not labels:
        return
    w, h = 720, 360
    ml, mr, mt, mb = 100, 40, 40, 60
    pw, ph = w - ml - mr, h - mt - mb
    groups = ("real_s", "user_s", "sys_s")
    vals = {g: [best[k][i] for k in labels] for i, g in enumerate(groups)}
    vmax = max(max(vals[g]) for g in groups) * 1.15 or 1.0
    gw, bw = pw / len(labels), pw / len(labels) / (len(groups) + 1)

    lines = _svg(w, h, "Wall-clock time per benchmark (lower is better)")
    lines.append(f'<line x1="{ml}" y1="{mt}" x2="{ml}" y2="{mt+ph}" stroke="#333"/>')
    lines.append(f'<line x1="{ml}" y1="{mt+ph}" x2="{ml+pw}" y2="{mt+ph}" stroke="#333"/>')
    for k in range(6):
        frac = k / 5
        y = mt + ph - ph * frac
        lines.append(f'<line x1="{ml}" y1="{y}" x2="{ml+pw}" y2="{y}" stroke="#e5e7eb"/>')
        lines.append(_text(ml - 8, y + 4, f"{vmax*frac:.1f}s", text_anchor="end", fill="#444"))
    for gi, g in enumerate(groups):
        for wi, name in enumerate(labels):
            v = vals[g][wi]
            bh = (v / vmax) * ph
            x = ml + wi * gw + (gi + 0.5) * bw
            y = mt + ph - bh
            lines.append(_rect(x, y, bw * 0.9, bh, PALETTE[gi]))
            lines.append(_text(x + bw * 0.45, y - 4, f"{v:.2f}", text_anchor="middle"))
    for wi, name in enumerate(labels):
        lines.append(_text(ml + (wi + 0.5) * gw, mt + ph + 18, name, text_anchor="middle"))
    for gi, g in enumerate(groups):
        lines.append(_rect(ml + gi * 110, h - 34, 12, 12, PALETTE[gi]))
        lines.append(_text(ml + gi * 110 + 18, h - 24, g))
    _write(out, lines)


def _render_rss(best, out: Path) -> None:
    labels = list(best)
    if not labels:
        return
    w, h = 560, 320
    ml, mr, mt, mb = 100, 40, 40, 60
    pw, ph = w - ml - mr, h - mt - mb
    vals = [best[k][3] / 1024 for k in labels]
    vmax = max(vals) * 1.2 if vals else 1.0
    bw = pw / len(labels) * 0.6
    gap = pw / len(labels) * 0.4

    lines = _svg(w, h, "Peak RSS per benchmark (MiB)")
    lines.append(f'<line x1="{ml}" y1="{mt}" x2="{ml}" y2="{mt+ph}" stroke="#333"/>')
    lines.append(f'<line x1="{ml}" y1="{mt+ph}" x2="{ml+pw}" y2="{mt+ph}" stroke="#333"/>')
    for k in range(6):
        frac = k / 5
        y = mt + ph - ph * frac
        lines.append(f'<line x1="{ml}" y1="{y}" x2="{ml+pw}" y2="{y}" stroke="#e5e7eb"/>')
        lines.append(_text(ml - 8, y + 4, f"{vmax*frac:.0f}", text_anchor="end", fill="#444"))
    for wi, name in enumerate(labels):
        v = vals[wi]
        bh = (v / vmax) * ph
        x = ml + wi * (bw + gap) + gap / 2
        y = mt + ph - bh
        lines.append(_rect(x, y, bw, bh, PALETTE[1]))
        lines.append(_text(x + bw / 2, y - 4, f"{v:.1f}", text_anchor="middle"))
        lines.append(_text(x + bw / 2, mt + ph + 18, name, text_anchor="middle"))
    _write(out, lines)


# --- Hotspot + flame from Apple sample files -------------------------
SAMPLE_ROW = re.compile(r"\s+(\S.*?)\s+\(in (\S+)\)\s+(\d+)\s*$")


def _parse_sample(path: Path) -> list[tuple[str, int]]:
    """Return [(clean_symbol, samples), ...] from the 'Sort by top of stack' table."""
    text = path.read_text().splitlines()
    try:
        start = next(i for i, ln in enumerate(text) if "Sort by top of stack" in ln)
    except StopIteration:
        return []
    out: list[tuple[str, int]] = []
    for ln in text[start + 1 :]:
        if not ln.strip() or ln.startswith("Binary Images"):
            break
        m = SAMPLE_ROW.match(ln)
        if m:
            out.append((clean(m.group(1)), int(m.group(3))))
    return out


_BUCKETS = [
    ("Bus Mutex lock/unlock", re.compile(r"pthread_mutex_|DYLD-STUB\$\$pthread_mutex_")),
    ("Mtimer::check_timer",   re.compile(r"mtimer.*check_timer|check_timer")),
    ("MMU::checked_read",     re.compile(r"checked_read")),
    ("MMU::access_bus",       re.compile(r"access_bus")),
    ("Trap::commit_trap",     re.compile(r"commit_trap")),
    ("Mtimer::tick",          re.compile(r"mtimer.*tick")),
    ("PLIC evaluate+tick",    re.compile(r"plic.*(evaluate|tick)")),
    ("CPU main loop",         re.compile(r"(^|::)(main|run)$")),
]


def _bucketize(rows: list[tuple[str, int]]) -> list[tuple[str, float]]:
    total = sum(n for _, n in rows) or 1
    buckets: dict[str, int] = {name: 0 for name, _ in _BUCKETS}
    other = 0
    for sym, n in rows:
        for name, pat in _BUCKETS:
            if pat.search(sym):
                buckets[name] += n
                break
        else:
            other += n
    result = [(k, 100 * v / total) for k, v in buckets.items() if v > 0]
    if other:
        result.append(("Other", 100 * other / total))
    result.sort(key=lambda kv: -kv[1])
    return result


def _render_pie(rows: list[tuple[str, float]], title: str, out: Path) -> None:
    if not rows:
        return
    total = sum(p for _, p in rows) or 1.0
    w, h = 640, 400
    cx, cy, r = 180, h / 2, 150
    theta = -math.pi / 2
    lines = _svg(w, h, title)
    for i, (label, pct) in enumerate(rows):
        frac = pct / total
        dt = frac * 2 * math.pi
        x1, y1 = cx + r * math.cos(theta), cy + r * math.sin(theta)
        theta2 = theta + dt
        x2, y2 = cx + r * math.cos(theta2), cy + r * math.sin(theta2)
        large = 1 if dt > math.pi else 0
        color = PALETTE[i % len(PALETTE)]
        lines.append(
            f'<path d="M {cx} {cy} L {x1:.2f} {y1:.2f} '
            f'A {r} {r} 0 {large} 1 {x2:.2f} {y2:.2f} Z" '
            f'fill="{color}" stroke="white" stroke-width="1.5"/>'
        )
        ly = 60 + i * 26
        lines.append(_rect(360, ly - 12, 14, 14, color))
        lines.append(_text(382, ly, f"{label} — {pct:.1f}%"))
        theta = theta2
    _write(out, lines)


def _render_selftime_bar(rows: list[tuple[str, int]], title: str, out: Path) -> None:
    """Ranked self-time bar (NOT a flamegraph — no call-stack depth)."""
    if not rows:
        return
    rows = sorted(rows, key=lambda r: -r[1])
    total = sum(n for _, n in rows) or 1
    w, h = 980, 340
    ml, mr, mt = 10, 10, 40
    pw = w - ml - mr
    lines = _svg(w, h, title)
    lines.append(
        _text(w / 2, 36,
              f"each block width ∝ self-time samples; total {total}",
              text_anchor="middle", fill="#666"))
    x = ml
    for i, (sym, n) in enumerate(rows):
        bw = (n / total) * pw
        color = PALETTE[i % len(PALETTE)]
        lines.append(_rect(x, mt + 10, bw, 80, color, stroke="white", stroke_width=1))
        if bw > 110:
            lines.append(_text(x + 6, mt + 52, f"{sym} ({100*n/total:.1f}%)",
                               fill="white", font_weight="bold"))
        x += bw
    col_w = pw / 2
    for i, (sym, n) in enumerate(rows):
        r_idx, c_idx = i // 2, i % 2
        yy = mt + 110 + r_idx * 18
        xx = ml + c_idx * col_w
        lines.append(_rect(xx, yy - 9, 10, 10, PALETTE[i % len(PALETTE)]))
        lines.append(_text(xx + 14, yy, f"{sym} — {n} samples ({100*n/total:.1f}%)"))
    _write(out, lines)


# --- Dispatcher ------------------------------------------------------
def _default_dir() -> Path:
    root = PROJECT_ROOT / "docs" / "perf"
    dated = sorted(
        (p for p in root.iterdir() if p.is_dir() and re.fullmatch(r"\d{4}-\d{2}-\d{2}", p.name)),
        key=lambda p: p.name,
    )
    if not dated:
        sys.exit("no docs/perf/<YYYY-MM-DD>/ directory found")
    return dated[-1]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", type=Path, default=None,
                    help="perf run directory (default: newest docs/perf/<date>)")
    args = ap.parse_args()

    run_dir = (args.dir or _default_dir()).resolve()
    data = run_dir / "data"
    graphics = run_dir / "graphics"
    graphics.mkdir(parents=True, exist_ok=True)

    print(f"[render] run  = {run_dir.relative_to(PROJECT_ROOT)}")
    print(f"[render] data = {data.relative_to(PROJECT_ROOT)}")

    best = _load_bench(data)
    _render_time(best, graphics / "bench_time.svg")
    _render_rss(best,  graphics / "bench_rss.svg")

    for sample in sorted(data.glob("*.sample.txt")):
        name = sample.stem.split(".sample")[0]
        rows = _parse_sample(sample)
        _render_pie(_bucketize(rows),
                    f"CPU self-time — {name} (bucketed)",
                    graphics / f"hotspot_{name}.svg")
        _render_selftime_bar(
            rows,
            f"Ranked self-time — {name} (Apple sample @ 1 kHz; NOT a flamegraph)",
            graphics / f"selftime_{name}.svg")


if __name__ == "__main__":
    main()
