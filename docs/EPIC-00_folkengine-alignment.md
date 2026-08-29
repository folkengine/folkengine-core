# EPIC — Align `folkengine` (folksonomy kernel) with the FolkEngine substrate

Status: proposed. Target repo: `folkengine/folkengine` @ `master` (2 commits, 2026-08-28).
Governing documents: `ARCHITECTURE.md`, `SPEC-cas-and-sigsuite.md`,
`SPEC-rebuild-and-rotation.md`, `SPEC-canonical-encoding.md`, `FOLKENGINE_CHARTER.md`.

---

## Goal

The folksonomy kernel is structurally a `Ledger<E>` + `DerivedView` pair that does
not yet know it is one. This epic closes that gap: it makes the kernel's
transition contract, its encoding, its identity model, and its test surface
satisfy the substrate's invariants, so a `folkcore` shell can wrap it without
weakening any of them.

## Definition of done

1. `fold(empty, events) == next_state` is a CI-enforced property over every
   action arm.
2. Every state-changing decision, including curator membership, is expressible as
   a ledger entry.
3. There is exactly one normative encoding of every public type, reachable
   through one chokepoint, with pinned test vectors.
4. The pure dependency tree is still one line and every existing purity gate
   still passes.
5. The crate's license, MSRV, name, and repo home agree with the rest of the
   project and with themselves.

## Non-goals

- Signing, CAS, or transport inside the kernel. Those belong to the shell.
- Index structures / performance work. Correct first, per the README.
- Resolving Spike #1 (`quinn` vs libp2p) or Spike #3 (encrypted search).

## Verification note

Findings in Phase 0 were verified against the published repo. Tasks touching
`src/` and `wit/` are written as *determine and pin*, because those files were
not read when this epic was drafted. Confirm the current behaviour before
treating any of them as a defect report.

---

## Phase 0 — Blockers (land before anything depends on the crate)

### T-01 — Correct the license
**Size:** S · **Depends on:** — · **Verified defect**

`Cargo.toml` declares `license = "MIT OR Apache-2.0"` and the README repeats it,
but `LICENSE` is the verbatim GPL-3.0 text and GitHub is classifying the repo as
GPL-3.0. The whole license-cleanliness discipline in this project (Apache-2.0
`folkcore-gofish` specifically to avoid a GPL bridge from `pkcore`) depends on the
substrate being permissive. A GPL LICENSE file on the kernel silently defeats it.

**Acceptance**
- `LICENSE` removed; `LICENSE-MIT` and `LICENSE-APACHE` added.
- README license section names both files.
- GitHub's detected license reads `MIT OR Apache-2.0`.
- `cargo deny check licenses` passes with the declared expression.

### T-02 — Raise MSRV to 1.81
**Size:** S · **Depends on:** — · **Verified defect**

`rust-version = "1.80"`; `ARCHITECTURE.md` §8 sets the floor at 1.81, driven by
`frost-ed25519` v3. A kernel below the workspace floor is a future surprise, not
a freedom.

**Acceptance**
- `rust-version = "1.81"` in `Cargo.toml`.
- CI matrix pins 1.81 as the MSRV job and builds clean.

### T-03 — ADR: crate name and repo home
**Size:** S · **Depends on:** — · **Decision required**

`ARCHITECTURE.md` names the system FolkEngine and the substrate `folkcore`. The
crate currently called `folkengine` is neither; it is one domain kernel among
several, a sibling of `folkcore-gofish`. The top-level name is spent on a leaf.
Separately, the scaffold lives in the `folkengine` GitHub org while the rest of
the work lives in `ImperialBower`.

**Acceptance**
- ADR recorded (use `engineering:architecture` shape) with the decision and its
  consequences for crates.io, the WIT world name, and the org split.
- If renamed: crate, WIT world, README, and `ARCHITECTURE.md` §13 updated
  together in one commit.

### T-04 — Register the kernel in the architecture map
**Size:** S · **Depends on:** T-03

The crate does not appear in `ARCHITECTURE.md` §9 (proving grounds), §11 (stack
table), or §13 (document map). Until it does, it is undocumented infrastructure.

**Acceptance**
- §13 gains a row for the crate and for `FOLKENGINE_CHARTER.md`.
- §9 gains a proving-ground row: folksonomy kernel, role, status.
- §11 gains a "domain kernels" row naming the WIT boundary.

---

## Phase 1 — Make the kernel a ledger

### T-05 — Pin `fold(events) == next_state`
**Size:** M · **Depends on:** — · **Highest-leverage task in the epic**

`apply` returns `Transition { state, events }`. Nothing asserts the two halves
agree. This equation is the crate-level form of
`rebuild_is_deterministic_and_idempotent` (`SPEC-rebuild-and-rotation.md` §3.6),
and without it the state path and the event path can drift silently — which would
make the substrate's disposable-index invariant false at the domain layer.

**Acceptance**
- A pure `fold(state, &[FolkEvent]) -> Folksonomy` exists (kernel, not testkit).
- Property test: for every reachable state and every legal action,
  `fold(pre, transition.events) == transition.state`.
- Covers every `Action` arm including `Merge`, `Rename`, and rejected
  transitions (a rejection emits no events and mutates nothing — extends the
  existing `rejection_never_mutates` test).
- CI job fails on violation.

### T-06 — Curator membership as an action
**Size:** M · **Depends on:** T-05

`Folksonomy::empty(curators)` fixes the curator set outside the transition, so a
curator change escapes the event stream and is therefore unattributable. This
contradicts `ARCHITECTURE.md` §4.5, where `MembershipChange` is a first-class
self-governance entry.

**Acceptance**
- `Action::AddCurator` / `Action::RemoveCurator` (naming per charter conventions)
  with matching events.
- Authorization rule pinned by test: who may change the curator set in open mode
  vs curated mode, and what happens when the last curator is removed.
- `empty(curators)` retained only as a genesis convenience, documented as
  equivalent to folding the corresponding events over `empty(&[])`.
- T-05's property holds over the new arms.

### T-07 — Pin identity provenance for `TagId` / `ItemId` / `ActorId`
**Size:** M · **Depends on:** T-05 · **Determine and pin**

Entropy crates are banned, so ids cannot be minted randomly inside the kernel.
Determine the current mechanism and make it normative: either caller-supplied
(ids are shell input) or content-derived (a pure function of the defining
action). Both are defensible; the substrate needs to know which, because ledger
entries must be replayable to the same ids on every node.

**Acceptance**
- Documented rule stating where ids originate and why replay reproduces them.
- Test: replaying an event stream from `empty` on a fresh process yields
  identical ids (pairs naturally with `kata1-determinism`'s cross-process
  replayer).
- If content-derived: the derivation is a pure function with a pinned test
  vector, and collision behaviour is a typed error, not a silent overwrite.

### T-08 — Document `from_parts` + `validate` as the fold-cache load path
**Size:** S · **Depends on:** T-05

The shell load path is the exact place where a cached view could be mistaken for
truth. Say so in the type docs, in the substrate's vocabulary.

**Acceptance**
- Rustdoc on `from_parts` states that the loaded state is a cache, that the CAS
  event stream is authoritative, and that `validate` is a defect report on a
  cache, not a proof of provenance.
- `Defect` variants cross-referenced to `RebuildError` where they correspond.

---

## Phase 2 — One encoding, one chokepoint

### T-09 — Repair the YAML references in the SPECs
**Size:** M · **Depends on:** — · **Cross-repo · Verified defect**

`SPEC-cas-and-sigsuite.md` §2.4 defines `canonical_body` via
`serde_yml::to_string`, and `SPEC-rebuild-and-rotation.md` §3.4 repeats it in the
verified fold. The dCBOR (Gordian profile) decision supersedes this. Stale
normative text in a document that Part 3 depends on is more dangerous than an
open question, because it reads as settled.

**Acceptance**
- Both documents updated to route canonical bytes through the `canonical`
  module per `SPEC-canonical-encoding.md`.
- A changelog line in each noting the supersession and its date.
- Grep across all specs for `yml`/`yaml` returns only historical notes.

### T-10 — Make WIT normative and serde advisory
**Size:** M · **Depends on:** T-09

`wit/folkengine.wit` and the `serde` derives both describe every public type and
can disagree on field order, optionality, and discriminants. Two contracts means
the bytes for a value depend on which shell encoded it — and any digest over
those bytes inherits the ambiguity.

**Acceptance**
- Charter and rustdoc state that the WIT world is the contract and that serde
  carries no stability or canonicity guarantee.
- `serde` feature docs explicitly warn that serde output is not the canonical
  encoding and must not be signed.
- CI check that the WIT world still resolves (`wasm-tools`) and generates
  bindings (`componentize-py`) — already run, now stated as contract enforcement.

### T-11 — CDDL schemas and test vectors for kernel types
**Size:** L · **Depends on:** T-07, T-10

`SPEC-canonical-encoding.md` mandates CDDL plus real test vectors for core entity
types. The folksonomy types are core entity types the moment a shell writes them
to the CAS.

**Acceptance**
- CDDL for `Tag`, `Binding`, `Visibility`, `FolkEvent`, `Action`.
- At least one test vector per type: value, canonical dCBOR hex, BLAKE3 address.
- Vectors live in the repo and are asserted by a test, not pasted into a doc.
- Map-key ordering follows the single resolved profile; a second profile in the
  fixtures is a CI failure.

### T-12 — `folkengine-shell-cbor` (or equivalent) as the encoding chokepoint
**Size:** M · **Depends on:** T-11

The kernel must stay format-free. Canonical encoding therefore lives in a
companion crate that depends on the kernel, never the reverse.

**Acceptance**
- New crate depending on the kernel + dCBOR, exposing encode/decode for every
  public type.
- Kernel's pure `cargo tree --no-default-features` still exactly one line.
- Round-trip property test: `decode(encode(v)) == v` over testkit-generated
  values (T-13).
- Encoding is idempotent at the byte level: `encode(decode(encode(v))) ==
  encode(v)`.

---

## Phase 3 — Testkit and conformance

### T-13 — `folkengine-testkit` companion crate
**Size:** L · **Depends on:** T-05

Per the kernel-testkit pattern: the primitive is a seeded *legal action
sequence* folded through `apply`, not a hand-assembled state. Hand-built states
are type-valid but may be unreachable, which makes every property tested against
them suspect.

**Acceptance**
- Trace generators take a seed; no ambient entropy or clocks anywhere.
- Generators build states by folding actions through the kernel's own `apply`.
- Failures reproduce from a `(seed, trace)` pair.
- Kernel does not depend on the testkit; the pure tree is unchanged.
- Four labelled kinds present: arbitrary-valid, adversarial/edge, golden
  fixtures, named scenario traces.

### T-14 — Texture map for the folksonomy state space
**Size:** M · **Depends on:** T-13

**Acceptance**
- `TEXTURE_MAP.md` naming the qualitative regions — candidates: deep hierarchy,
  wide-flat vocabulary, diamond (poly-hierarchy with shared ancestors),
  near-cycle, heavy alias load, open vs curated, single-tagger vs contested
  binding, post-merge, mixed visibility.
- Each texture has a classifier and a generator.
- Round-trip law tested: `classify(generate(texture, seed))` contains `texture`.

### T-15 — State coverage report and CI gate
**Size:** M · **Depends on:** T-14

**Acceptance**
- Test-time hook classifies every pre-state; report emits textures hit / defined,
  transition coverage (texture × action class), and the untextured rate.
- Both metrics reported; either alone is gameable.
- CI publishes the report; gate threshold agreed and recorded in the ADR.

### T-16 — Golden fixtures as the cross-language contract
**Size:** M · **Depends on:** T-11, T-13

**Acceptance**
- Canonical serialized traces + terminal states, hash-pinned.
- A fixture change requires an explicit version bump, not a silent re-record.
- Fixtures are the artifact a second implementation is judged against.

### T-17 — Second implementation and conformance run
**Size:** L · **Depends on:** T-16 · **Closes the README's "Not yet"**

**Acceptance**
- A Python (or other non-Rust) implementation driven from the same WIT world.
- Conformance harness replays every golden trace against both implementations
  and diffs terminal state and event order.
- Divergence is a hard CI failure with the diverging trace printed.

---

## Phase 4 — Substrate integration

### T-18 — wasm32-wasip2 guest build in CI
**Size:** M · **Depends on:** T-10 · **Closes the README's "Not yet"**

The `pkcore` → `pkgto-web` pattern already proves the compilation model; this
applies it. The blocker recorded on 2026-08-28 was environmental (no wasm std in
the drafting sandbox), not architectural.

**Acceptance**
- `cargo component build` produces a component in CI.
- The Rust guest imports nothing (verify with `wasm-tools component wit`).
- Component artifact attached to CI runs.

### T-19 — `FolkEvent` as an `Entry<E>` payload in a `folkcore` shell
**Size:** L · **Depends on:** T-05, T-12

The integration proof: kernel events become signed, content-addressed ledger
entries, and the kernel state becomes a `DerivedView` recomputed by
`fold_verified`.

**Acceptance**
- Shell implements `EntryPayload` for `FolkEvent` (returns `None` for
  `as_group_rotation` — folksonomy events are data, never governance).
- `TagView: DerivedView<Entry = FolkEvent>` whose `apply` delegates to the
  kernel's fold.
- `fold_verified` over a fixture DAG reproduces a kernel-computed state exactly.
- `rebuild_index` includes the folksonomy view and the §3.6 determinism test
  passes with it present.

### T-20 — Reconcile `view_for` with envelope encryption
**Size:** M · **Depends on:** T-19 · **Design task**

`view_for(actor)` is a *projection*: it filters what an actor may see from a
state the caller already holds in plaintext. `ARCHITECTURE.md` §4.7 says the
platform holds only ciphertext. These are compatible only if the boundary is
stated: the kernel projects within a trust domain that has already decrypted, and
the DEK boundary is what keeps a non-grantee from reaching the kernel at all.
Left unstated, `view_for` reads as an access-control mechanism, which it is not.

**Acceptance**
- Written boundary statement in the charter and in `ARCHITECTURE.md`.
- Explicit note that `view_for` is not a confidentiality control and must never
  be the only thing between an actor and data they cannot decrypt.

### T-21 — `AccessView` fold — retire OpenFGA (Spike #2)
**Size:** L · **Depends on:** T-19 · **Resolves an open spike**

The spike asks for a concrete query UCAN structurally cannot answer. The likely
candidate is the reverse query, "who can currently see item X." In this
architecture that is a derived view: fold grant and revocation entries into a
reverse index cached in SQLite, rebuildable from the CAS like everything else.
Building it as a fold keeps one authorization authority and makes the answer
independently verifiable.

**Acceptance**
- `AccessView: DerivedView` answering the reverse query.
- Written finding: either a query the fold provably cannot answer (keep the
  spike open), or none (cut OpenFGA and mark §11 superseded).
- `ARCHITECTURE.md` §12 item 2 updated with the outcome either way.

---

## Deferred (registered, not scheduled)

| Item | Trigger to schedule |
|---|---|
| Index structures for `resolve` / `descendants` | A shell reports a measured problem |
| Tag subject rights (§7) as kernel actions | Autotagging pipeline reaches implementation |
| Revocation of bindings as superseding entries | After T-19 lands and Gordian revocation semantics resolve |
| Lift "private during, auditable after" into `folkcore` | Spike #8 |

---

## Sequencing

```
T-01 ─┐
T-02 ─┼─► T-03 ─► T-04
      │
T-05 ─┼─► T-06 ─┐
      ├─► T-07 ─┼─► T-11 ─► T-12 ─┐
      └─► T-08  │                 │
                │                 ├─► T-19 ─┬─► T-20
T-09 ─► T-10 ───┘                 │         └─► T-21
      └────────► T-18             │
                                  │
T-05 ─► T-13 ─► T-14 ─► T-15      │
           └──► T-16 ─► T-17 ─────┘
```

**Critical path:** T-05 → T-07 → T-11 → T-12 → T-19. Everything that makes the
kernel a substrate participant runs through the fold invariant, so T-05 is the
first task worth doing and the one to do properly.

**Parallel now:** Phase 0 in full, plus T-09 (spec repair, different repo,
different reviewer).
