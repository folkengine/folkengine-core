# ADR-0001 — The crate keeps the name `folkengine`; the repo stays in the `folkengine` org

Status: accepted, 2026-08-29.
Supersedes: nothing. Closes: EPIC-00 T-03.
Decided by: repository owner.

---

## Context

EPIC-00 T-03 raised the crate's name as a blocker, on this argument:
`ARCHITECTURE.md` names the system **FolkEngine** and its substrate
**folkcore**, so a crate called `folkengine` is neither — it is one domain
kernel among several, a sibling of `folkcore-gofish`, and the top-level name is
spent on a leaf. Since crates.io names are first-come and permanent, publishing
under `folkengine` would burn the system's name on one of its parts.

Separately, the repository lives in the `folkengine` GitHub org while the rest
of the work lives in `ImperialBower`.

## Decision

**The crate keeps the name `folkengine`.**

The epic's premise is rejected. This crate is not a leaf: it is the core library
of the work, and the core library is the right thing to own the project name. A
project whose central library shares its name is the normal case, not a
collision — the name points at the thing people actually depend on, and the
surrounding parts take qualified names.

Everything that follows from the name is therefore unchanged:

```wit
package folkengine:folksonomy@0.1.0;
world folkengine { export kernel; }
```

The WIT world stays `folkengine`, the file stays `wit/folkengine.wit`, the Rust
identifier stays `folkengine`, and `FOLKENGINE_CHARTER.md` keeps its name and
its prose.

**The repository stays in the `folkengine` GitHub org.** The org name is read as
the project, this crate is its core, and `ImperialBower` keeps the poker work.
The split is deliberate, not an accident of where a scaffold landed.

## Consequences

- `folkengine` on crates.io belongs to this crate. That is now a decision rather
  than a default, which is the point of writing it down: the name is claimed on
  purpose, at the moment it was cheapest to change.
- No source, contract, or documentation churn. `use folkengine::…` stays,
  the CI contract job still runs
  `componentize-py -d wit/folkengine.wit -w folkengine bindings`, and the
  charter needs no pass.
- **`ARCHITECTURE.md` now has a vocabulary problem to settle, and it is the only
  live consequence of this ADR.** That document uses *FolkEngine* for the whole
  system and *folkcore* for the substrate. If the core library is `folkengine`,
  then either the system name and the core library name deliberately coincide —
  the usual arrangement, and the one this ADR implies — or the system needs a
  different word. Related: whether `folkcore` remains a distinct substrate name
  once `folkengine` is the core. **Open, assigned to EPIC-00 T-04**, which is the
  task that edits `ARCHITECTURE.md` §9, §11 and §13 anyway. This ADR does not
  decide it, because it is a decision about the other repository's vocabulary.
- `Cargo.toml`'s `repository` key continues to read
  `https://github.com/folkengine/folkengine`, which is correct.

## Alternatives considered

**Rename to `folksonomy-kernel`.** Names both the domain and the pattern, and
frees `folkengine` for the system. Implemented in full during the 2026-08-29
session — crate, WIT world, file, README, tests, CI — and then reverted, because
it answers a question the owner does not have: it treats the crate as one part
among several when it is the core. The mechanical cost was low and the change is
reversible in either direction while the crate is unpublished, so the decision
rests on what the crate *is*, not on migration cost.

**`folkcore-folksonomy`,** matching `folkcore-gofish`. Rejected on a second
ground that survives regardless of the naming question: `folkcore` is the
*substrate* — a `Ledger<E>` plus `DerivedView`, with signing, CAS and transport.
This crate has none of that and must not depend on it; the dependency runs the
other way, from the shell (EPIC-00 T-19) inward. A `folkcore-` prefix would
assert a relationship the purity gates exist to forbid.

**`folksonomy` alone.** Silent about the pattern, and the pattern is the
load-bearing claim: a shell may treat this crate as a pure function.
