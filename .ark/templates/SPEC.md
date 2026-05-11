[**Goals**]

> What the feature does. One line per bullet, ≤80 chars, verb-led, capability-oriented (the user-visible *what*, not the *how*). Soft cap: 5 goals. If you have more, you are listing implementation steps — promote them to Constraints or drop them.
>
> Good: `G-1: ark context prints a JSON snapshot of git + tasks + specs.`
> Bad:  `G-1: Two flags control output: --scope {session|phase} and --for {design|...} (required iff --scope=phase). Clap rejects mismatched combinations.`  ← this is implementation detail; belongs in Constraints.

- G-1:
- G-2:
- G-3:

[**Non-goals**]

> Only list a non-goal when a reasonable reader would assume it is in scope. Skip "no X" bullets where X is far outside the feature's natural reach. Soft cap: 3.
>
> Good: `NG-1: No mutation — read-only command.`
> Bad:  `NG-1: No multi-developer concepts. NG-2: No monorepo aggregation.`  ← nobody asked for those.

- NG-1:

[**Architecture**]

> Module / file layout, with a one-line note per file describing its role. Diagrams are welcome; prose narration is not. If the diagram says everything, no prose is needed.

```
<directory tree or component diagram>
```

[**Data Structure**]

> Public types only. Field names + types + a one-line comment when the meaning is non-obvious. No bodies, no derived methods unless they are part of the API.

```rust
struct ...
enum ...
trait ...
```

[**API Surface**]

> Public function signatures and their one-line semantics. No bodies. If a function's behaviour is captured by its signature + name, omit the comment.

```rust
fn ...
```

[**Constraints**]

> Invariants the implementation must hold. One declarative sentence each, ≤120 chars. Cite the source of truth (a constant, a test, a file path) when one exists. **No paragraphs, no multi-sentence rationale** — if a rule needs justification, the *why* belongs in PLAN's Trade-offs, not here.
>
> Good: `C-1: ark context emits exactly one stdout write per invocation.`
> Bad:  `C-1: ark context emits exactly one stdout write per invocation: JSON via a single pre-rendered string + trailing newline, text via a single Display write + trailing newline. No interspersed debug prints.`  ← the elaboration is the *how*; the constraint is the first sentence.

- C-1:
- C-2:

[**CHANGELOG**]

> Appended only when a later task modifies this SPEC. New SPECs (extracted from a deep-tier PLAN at commit) start with this section empty.

- `<YYYY-MM-DD>` `<task-slug>`: <one line: what changed and why>
