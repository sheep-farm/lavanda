# Development model

lavanda is developed under a deliberate model the author calls **convergent
evolution**. It is worth stating explicitly, because it inverts the usual
expectation that a single project is the one true place to contribute.

## The idea

Instead of one repository where everyone coordinates, lavanda evolves through
**fork-and-diverge + selective reintegration**:

1. **Fork freely.** Anyone (including the author) may fork lavanda and take it
   in their own direction without asking. No issue, no RFC, no permission. The
   fork is a cheap, reversible experiment.
2. **Diverge in isolation.** Each fork evolves on its own under similar
   pressures — the Omarchy / Wayland / keyboard-first niche — and tends to grow
   similar features by independent paths.
3. **Reintegrate selectively.** When a fork does something good, the ideas (and,
   when it makes sense, the code) are folded back. Sometimes that is a verbatim
   transfer; more often it is a re-implementation of the same trait.

The sibling project [OmaTunes](https://github.com/Balthazzahr/omatunes) is the
first instance: it forked from an early lavanda, explored playlists, an indexed
library, play counts and a richer tag editor, and lavanda 1.0.1 folded those
ideas back in. The two now evolve as separate "echoes" of a shared origin and
may keep cross-pollinating in both directions.

## Why this instead of one shared repo

Two people coordinating in one tree generates transaction cost and noise:
merge conflicts, review latency, design-by-committee. Forking is permissionless
— it removes the coordination cost up front and lets selection decide *after the
fact* which mutation was worth keeping. Each repo is an independent, low-cost
trial; the good variants survive by being copied, not by being approved. No one
needs to know the other's plan, only to see the result and adopt what works.

## The borrowed vocabulary (for the curious)

The biology is closer to **allopatric speciation with gene flow** than to
convergent evolution in the strict sense:

- **Allopatric speciation** — a population splits behind a barrier (here, the
  fork + separate repositories) and each side evolves independently.
- **Parallel evolution** — *related* lineages (a shared git ancestor) grow
  similar traits under similar selection pressures.
- **Introgression / horizontal transfer** — occasional code copied across the
  barrier (e.g. `persist.rs`, the `Track` model came from OmaTunes).

A caveat on the name: in software writing, "convergent evolution" is already
used for something *different* — two unrelated practitioners independently
arriving at the same solution, with no shared ancestry and no exchange. That
usage is the biologically strict one. What happens here is the opposite: a
shared ancestor plus deliberate gene flow. We keep "convergent evolution" as a
deliberately loose working name because it captures what an outside observer
sees in the end — two morphologically similar applications that reached that
point by partly independent routes.

## Prior art

Neither the mechanism nor most of the terms are new. The empirical software
literature distinguishes **divergent forks** (a fork that takes its own
direction, not aiming to merge back) from **contributing forks**, and treats
constructive divergence as **positive divergence**. It also documents the
"introgression" step directly: code is routinely propagated between forks
*outside* the pull-request mechanism, plain `git` to plain `git` — along with
the failure mode this model must guard against (fixes that land in one fork and
silently never reach the other). What is unusual here is only the **normative
stance**: treating fork-and-diverge as the *preferred* path rather than a
fallback, which inverts the canonical fork-and-pull model.

## The one rule: traceability

The model only stays healthy if every lineage remembers where each gene came
from. Introgression is fine — appropriation is not, and the only thing that
separates them is attribution.

- When you transfer code across forks, **say so** (a commit message, a comment,
  or the `## Lineage` section of the README).
- Keep licenses compatible. lavanda and OmaTunes are both MIT.
- Never let a repo lose the memory of its origin; that is what turns healthy
  gene flow into something an outsider could mistake for theft.

## How to contribute

Both paths are welcome, but they are not equal in spirit:

- **Preferred — fork and diverge.** Build your own echo. If it grows something
  good, open an issue describing it (or just let it be noticed). Reintegration
  is pull-based.
- **Also fine — pull requests.** Small, focused fixes (bugs, docs, packaging)
  are easiest as PRs against `dev`.

## Practical notes

- **Branches.** Work lands on `dev`; `master` holds the latest release. `dev`
  carries the next `-dev` version; `master` carries the released version.
- **AUR packaging.** Prepared but unpublished PKGBUILDs live on the `packaging`
  branch so `master`/`dev` stay clean until lavanda is actually on the AUR.
- **Build & run.**
  ```bash
  cargo build --release
  ./target/release/lavanda
  ```
- **Architecture.** See the *Architecture* section of the README for the source
  layout. Library state (likes, play counts, playlists, recently played, column
  layout) is persisted as JSON in `~/.config/lavanda/db.json`.
