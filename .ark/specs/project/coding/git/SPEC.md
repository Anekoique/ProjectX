# Git Guidelines

> Rules for commit messages and branch hygiene. Astervisor follows [Conventional Commits 1.0.0](https://www.conventionalcommits.org/); commit messages drive changelog generation, SemVer decisions, and review focus.

## R1 — Every commit message follows the conventional structure

**Applies to:** all commits
**Evidence:** VERIFY

Shape: `<type>[optional scope]: <description>`, blank line, optional body, blank line, optional footers. A commit with no body and no footers is a single line.

```text
# Bad — multi-line subject, no type
Add ability to parse arrays
in the configuration file parser

# Good — single-line, no body
docs: correct spelling of CHANGELOG

# Good — body and footer
feat(parser): add ability to parse arrays

The parser now accepts bracketed list literals at the top level
of a configuration file, in addition to objects.

Refs: #42
```

## R2 — The `<type>` is one of the fixed set, lowercase

**Applies to:** all commits
**Evidence:** VERIFY

Allowed types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore`, `style`, `revert`. Pick the type that describes the *primary* intent. If a commit splits across types, split the commit. Do not invent new types.

```text
# Bad — invented type
enhancement(mm): improve frame allocator

# Bad — uppercase
Feat(mm): add frame allocator

# Good
feat(mm): add frame allocator
```

## R3 — Add a scope when it sharpens the change beyond the type alone

**Applies to:** all commits
**Evidence:** VERIFY

A scope is an optional noun in parentheses naming the area touched. Use it when it adds information beyond the type. A scope must be a single token — if you need a slash or comma, the commit is too broad. Prefer scopes that name the directory or subsystem (`ostd`, `visor`, `mm`, `task`, `vsdk`, `survey`), not abstract categories (`backend`, `core`).

```text
# Bad — multi-token scope
feat(mm/frame): add slot allocator
feat(backend): wire up VirtIO

# Good
feat(mm): add frame slot allocator
feat(virtio): wire up block device
ci: enable license-eye on pull_request
```

## R4 — Descriptions are imperative, lowercase, no terminal period

**Applies to:** all commits
**Evidence:** VERIFY

The description after the colon answers "what does this commit do?" — imperative mood ("add", not "added"), starts lowercase (unless first word is a proper noun or identifier), no trailing period, fits ~72 characters so `git log --oneline` stays readable.

```text
# Bad — past tense
feat(mm): added frame metadata slot allocator

# Bad — capitalized, ends with period
fix(task): Release kernel stack on panic path.

# Bad — describes the file rather than the change
chore: ostd/mm/frame.rs

# Good
feat(mm): add frame metadata slot allocator
fix(task): release kernel stack on panic path
```

## R5 — The body explains why, not what

**Applies to:** all commits with non-trivial change
**Evidence:** VERIFY

The body's job is context the diff cannot give: motivation, alternatives considered, the constraints that ruled the chosen approach. Wrap body lines at ~72 characters. Do not paste full diffs or stack traces — link to issue or PR numbers in footers instead.

```text
# Bad — body restates the diff
fix(mm): change shootdown order

Moved the IPI before the gencount update. Changed both lines.

# Good
fix(mm): prevent racing of TLB shootdown requests

The previous shootdown path issued an IPI before recording the
generation count, so a concurrent unmap on another CPU could
believe its own shootdown had completed when it had only seen
the earlier request.
```

## R6 — Footers carry structured metadata as `Token: value`

**Applies to:** all commits
**Evidence:** VERIFY

Footers go after a blank line at the end of the message. Each footer is one line; the token uses `-` instead of spaces (so it is distinguishable from a wrapped body line). Recognised tokens: `Refs`, `Closes`, `Reviewed-by`, `Co-authored-by`, `BREAKING CHANGE` (or `BREAKING-CHANGE`).

```text
# Bad — footer mixed into body
fix(mm): handle null page table

This was found in PR #45.

# Good
fix(mm): handle null page table

The fault handler dereferenced the page table without checking
for null, crashing on early-boot pages.

Closes: #45
Refs: #42
```

## R7 — Breaking changes are marked with `!` before the colon or a `BREAKING CHANGE:` footer

**Applies to:** commits that break a public API
**Evidence:** VERIFY

Either form bumps the MAJOR version. Use `!` for short, self-explanatory breaks; use the footer when the break needs prose to explain. Combining both is allowed.

```text
# Bad — breaking change unmarked
feat(mm): replace FrameAllocOptions with explicit constructors

# Good — option A: ! prefix
feat(mm)!: replace FrameAllocOptions builder with explicit constructors

# Good — option B: footer
feat(mm): rework frame allocator API

The new API exposes typed and untyped allocation as separate
entry points instead of a runtime-checked options struct.

BREAKING CHANGE: FrameAllocOptions has been removed. Callers
must migrate to FrameAllocator::alloc_typed and ::alloc_untyped.
```

## R8 — One logical change per commit

**Applies to:** all commits
**Evidence:** VERIFY

Granularity test: "Could this commit be reverted on its own without leaving the tree in a half-broken state?" If the answer is no, split it. If two parts only make sense together (a function and its single caller, both new), keep them in one commit.

```text
# Bad — one commit doing two things
fix(mm): handle null page table; refactor heap into separate file

# Good — split into two
fix(mm): handle null page table
refactor(mm): move heap into a sibling module
```

## R9 — PRs use squash-and-merge; the squashed commit follows this spec

**Applies to:** all PR merges to `main`
**Evidence:** VERIFY

Inside a feature branch, intermediate WIP commits are fine — GitHub squashes them on merge. The final squashed commit message is what lands on `main`'s history, so it must be cleaned up at merge time: right type, right scope, right body, right footers.

```text
# Bad — squashed commit retains WIP messages
fix stuff

wip

address review

# Good — squashed commit cleaned up at merge
feat(visor): introduce vCPU context save/restore

The save/restore path is the unsafe shim that the cooperative
scheduler calls when handing control between domains.
```

## R10 — Reverts use the `revert` type and cite the reverted commit

**Applies to:** commits that undo earlier work
**Evidence:** VERIFY

A `revert:` commit's SemVer impact mirrors the impact of the commit it reverts. Cite the reverted commit's hash and subject in the description or body.

```text
# Bad — bare revert message
Revert "feat(mm): add frame metadata slot allocator"

# Good
revert: feat(mm): add frame metadata slot allocator

The slot allocator caused contention under SMP because of the
shared free-list; revert until a per-CPU design lands.

Refs: 676104e
```

## R11 — Commit type honestly maps to SemVer impact

**Applies to:** all commits to `main`
**Evidence:** VERIFY

`feat:` claims a MINOR-version-worthy change. `fix:` claims a PATCH-version-worthy change. Anything with `!` or `BREAKING CHANGE:` claims a MAJOR-version-worthy change. When in doubt about which type to use, ask which version bump the change deserves and pick the type that maps to it.

```text
# Bad — feat: for a refactor with no new capability
feat(mm): rename FrameAlloc to PageAlloc

# Good — refactor: when no behavior changes
refactor(mm): rename FrameAlloc to PageAlloc
```
