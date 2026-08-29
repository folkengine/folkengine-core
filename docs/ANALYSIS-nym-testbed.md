# FolkEngine — Analysis: Nym as an In-the-Wild Test Bed

Status: analysis for review. Not a specification; produces no normative
statements about `folkcore`. Proposes one droppable kata-series project and
registers its outcomes against existing spikes. Depends on `ARCHITECTURE.md`
§4.3 (disposable-index invariant), §4.5 (verified fold), §5.2 (FFP), §5.3
(NATS truth discipline), §12 (spike register), and on
`SPEC-rebuild-and-rotation.md` §3.2 (`linearize`), §3.7 (edge cases).

> **Status conventions.** As in `ARCHITECTURE.md`: **[settled]**,
> **[evolving]**, **[SPIKE]**, plus **[verify]** for a factual claim about Nym
> that must be confirmed against source before it is relied on.

---

## 0. Summary

Nym is a poor *dependency* for FolkEngine and a good *adversary* for it. The
properties that make it risky as a component — unilateral pricing, a single
non-forkable network, a chain-published topology — are irrelevant to a test
bed, while the properties that make it hostile as a transport — unordered,
delayed, lossy, fixed-size delivery over strangers' nodes — are exactly the
conditions under which FolkEngine's transport-agnosticism claim has never been
observed.

Recommendation: run Nym as a **droppable kata** (`kata-nym-wild`), not as a
transport profile, not as a proving ground, and not as a constitutional claim.
Pass criteria are entirely about FolkEngine. Every failure is a spec gap.

---

## 1. The claim under test

FolkEngine asserts, across several documents, that the transport is untrusted
and irrelevant to correctness:

- Entries are self-verifying (`get_verified`, suite-committed signatures).
- Ordering is a pure function of the DAG — `(seq, ContentHash)` — not of
  arrival order (`SPEC-rebuild-and-rotation.md` §3.2).
- All readable state is a fold; the index is disposable (`ARCHITECTURE.md`
  §4.3).
- Notifications carry "something changed at head H," never "here is the
  thing" (§5.3).

Every existing test of these claims runs over a transport the project
controls: in-process iterators, or localhost. The claim's actual failure
domain — a transport that reorders, delays, drops, and fragments — has not
been exercised. **This is the gap the test bed closes.**

---

## 2. Why Nym, specifically

### 2.1 Shape of the adversary

Nym's raw Mixnet mode routes each message independently over a public network
of mixnodes with per-hop timing delays and cover traffic. From FolkEngine's
side this yields, by design and without any test harness:

| Hostile property | What it exercises |
|---|---|
| No ordering guarantee | `linearize` determinism under real disorder |
| Multi-second, variable latency | Axiom 4 ("revocation within one TTL") against a real clock |
| Loss without notification | Notification-never-truth (§5.3); convergence from CAS alone |
| Fixed-size Sphinx packets | Entry-size discipline; CAS chunking becomes non-optional |
| Concurrency made likely by delay | Concurrent rotation at merge points (§3.7) |
| Anonymous-sender delivery (SURBs) | Whether FFP payloads leak what the transport hid |
| Browser reaches network only via gateway | One-core-many-targets under a real WASM constraint |

### 2.2 Shape of what Nym does *not* provide

Nym is hostile to **availability and ordering**, not to **integrity**. It will
not forge a `GroupRotation`, replay a signature under the wrong `Domain`, or
return wrong bytes for a hash. Those adversaries stay where they are: in
fixtures and the three CI invariants of `SPEC-rebuild-and-rotation.md` §3.6.

Two constraints follow:

1. **Nym results are soak/integration evidence, never CI invariants.** A live
   network is non-deterministic. A flaky invariant test is worse than none for
   the properties FolkEngine pins.
2. **Nym's anonymity claims are not under test.** Whether the mixnet actually
   defeats a global passive adversary is Nym's research problem. The test bed
   measures FolkEngine's behaviour *given* a hostile transport, not the
   transport's privacy guarantees.

### 2.3 Cost profile

- SDK usage is currently free for development and testing. **[verify]** —
  confirm against `nym.com/docs/network` at kata start; this is the only
  economic assumption the kata makes.
- The Rust SDK's crates.io publication is paused; import from Git and pin a
  commit hash. **[verify]**
- Nothing in `folkcore` links the SDK. The kata is a leaf crate. When it has
  taught what it needed to, it is deleted — the same lifecycle as
  `kata0-ledger` and `kata1-determinism`.
- **License isolation.** The Nym monorepo is GPL-3.0; the SDK crate's own
  license must be checked. **[verify]** Because the kata is droppable and
  never a dependency of `folkcore` or `folkcore-gofish`, the Apache-2.0
  cleanliness of the substrate is unaffected regardless of the answer.

### 2.4 Why not the alternatives

| Alternative | Why it's weaker as a test bed |
|---|---|
| Toxiproxy / `tc netem` over localhost QUIC | Deterministic, scripted hostility. Tests what you thought to script. |
| Tor (via a SOCKS proxy) | Stream-oriented; preserves ordering within a circuit. Doesn't exercise the unordered-delivery path. |
| libp2p gossipsub on a testnet | Owned by the project; not "in the wild." Also entangles Spike #1. |
| A self-hosted mixnet | Anonymity set of one; no real timing hostility; significant operational cost. |

Nym is the only option that is simultaneously *public*, *unordered*, *lossy*,
*free to try*, and *Rust-native with a WASM path*.

---

## 3. Relationship to the earlier integration question

An earlier analysis considered Nym as an **optional transport profile** for
FFP and identified a real economic-dependency problem: pricing set unilaterally
by Nym Technologies, a single non-forkable anonymity network, routing governed
by the Nyx chain's staking economy, and a fiat→NYM linkability point in front
of an anonymity system. That analysis concluded Nym must never be load-bearing
for any axiom.

The test-bed framing **sidesteps that problem entirely**: a test bed promises
users nothing. It also reorders priorities. The native-primitives spike
(pull-from-rendezvous-CAS, sender-side batch-and-delay, blinded reply
addresses from FolkEngine's own key material) remains the right long-term
answer for metadata protection. The Nym kata is *orthogonal* to it: it tests
the substrate's robustness, not its privacy features, and its findings feed
the native-primitives work regardless of whether Nym is ever adopted.

---

## 4. Spikes this kata resolves or sharpens

Cross-referenced to `ARCHITECTURE.md` §12.

| Spike | Current state | What the kata contributes |
|---|---|---|
| #4 Linearization gap-detection policy | Missing `seq` is a hard error | Distinguishes *incomplete history* (refuse to produce a view) from *not yet arrived* (wait). Produces the missing spec. |
| #5 Concurrent rotation at merge points | Deferred to FFP merge story | Makes `RotationMismatch` a routine event; forces a first-class fork-resolution flow. |
| #6 NATS truth-discipline enforcement | A convention | If convergence survives ≥30% notification loss, the mechanical guard is "notification loss is injected in soak; convergence is measured." |
| (new) FFP anonymous-sender safety | Unspecified | Produces the list of FFP message types that are safe to send from an anonymous sender, and those that structurally are not. |
| (new) Axiom 4 TTL sizing | A number without a transport behind it | Either the TTL absorbs real mixnet latency or it is re-sized with rationale recorded. |
| (new) CAS chunking | Not a v1 concern | Fixed-size packets make it one. Produces a minimal chunking spec or a documented size bound on entries. |

---

## 5. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Kata findings get treated as CI invariants | §2.2 constraint 1. Findings are written into specs and *then* pinned as deterministic fixture tests. |
| SDK API churn during the kata (Git import, publication paused) | Pin commit. Time-box each EPIC. If the pin breaks, the kata ends and findings-to-date are written up. |
| Free-tier terms change mid-kata | Kata ends; findings-to-date are written up. No FolkEngine artifact depends on continuation. |
| Nym network instability confounds results | Log gateway/route metadata per message; separate "FolkEngine failed" from "delivery never happened." |
| GPL contamination | Leaf crate, never imported by `folkcore*`. Verified in §2.3. |
| Scope creep into "let's ship Nym" | This document's §0 and §3. Adoption is a separate decision with its own spike and exit trigger. |

---

## 6. Exit criteria for the kata as a whole

The kata is **complete** when all five EPICs below have either passed or
produced a written finding, and every finding has been either:

- resolved into a spec change (normative statement + deterministic test), or
- registered as a spike in `ARCHITECTURE.md` §12 with a trigger condition.

The kata is **abandoned** (not failed) if the SDK pin breaks irrecoverably or
free access ends. Abandonment still requires findings-to-date to be written up
against the spike register.

---

## EPICs

Numbering continues the kata series; concrete slot to be assigned when Kata 3–6
scope is fixed. Each EPIC has a goal, stories, acceptance criteria, and the
spec/spike it feeds. Stories are sized for a single working session unless
noted. Every EPIC produces a `FINDINGS.md` section; passing produces a
one-line entry, failing produces a spec-gap write-up.

---

### EPIC 0 — Harness

**Goal.** A minimal, droppable crate that sends and receives FolkEngine
`Entry<E>` values over Nym raw Mixnet mode, with enough instrumentation to
attribute every failure to either FolkEngine or the network.

**Stories.**

- [ ] 0.1 Create `kata-nym-wild` as a leaf crate. Import `nym-sdk` from Git at
  a pinned commit. Record commit hash, SDK license file contents, and the
  free-tier statement from Nym docs in `PROVENANCE.md`. **[verify]** items
  from §2.3 close here.
- [ ] 0.2 Define `Transport` trait in the kata (not in `folkcore`): `send(peer,
  bytes)`, `recv() -> Stream<bytes>`. Implement `NymTransport` (raw Mixnet)
  and `LoopbackTransport` (in-process, deterministic) behind it.
- [ ] 0.3 Wire canonical dCBOR encoding of `Entry<E>` as the only thing that
  crosses the transport. Payload type: a trivial `KataMove` enum sufficient to
  exercise `seq`, `prev`, and `GroupRotation`.
- [ ] 0.4 Per-message instrumentation: local send timestamp, receive
  timestamp, byte length, fragment count, SDK-reported route/gateway metadata
  where exposed. Write to an append-only local log (not the CAS).
- [ ] 0.5 Failure taxonomy: every observed failure is classified as
  `Delivery` (network), `Decode` (canonical encoding), `Verify` (signature /
  hash), or `Fold` (view divergence). No "unknown" bucket permitted.

**Acceptance.** Two native instances exchange a 10-entry chain over Nym and
both produce byte-identical `fold_verified` output. `LoopbackTransport` runs
the same test deterministically in CI.

**Feeds.** Nothing yet; this is scaffolding.

---

### EPIC 1 — Linearization under real disorder

**Goal.** Observe `linearize` and `fold_verified` producing byte-identical
output on instances that received the same DAG in different arrival orders,
with transient gaps.

**Stories.**

- [ ] 1.1 Three-instance sync: two native, one WASM-in-browser via a Nym
  gateway. Source instance publishes a 200-entry session including one
  `GroupRotation` at seq ≈ 100 and one merge point.
- [ ] 1.2 Each receiving instance records arrival order. Assert arrival orders
  differ across instances (if they don't, the test bed isn't hostile enough;
  increase entry rate or add a second author).
- [ ] 1.3 Each instance runs `rebuild_index` from its own CAS. Compare
  snapshot bytes across all three.
- [ ] 1.4 Gap behaviour: instrument the point at which `linearize` first
  observes a missing `seq`. Record how long the gap persisted before the entry
  arrived, and whether the current hard-error policy fired.
- [ ] 1.5 Prototype a two-state gap policy — `Pending { seq, since }` vs.
  `Missing { seq }` — with a configurable patience window. Measure false
  "Missing" rate as a function of window size.

**Acceptance.** Byte-identical rebuild across all three instances once all
entries have arrived. A written recommendation for the gap policy with the
patience-window data.

**Feeds.** Spike #4 (gap-detection policy) → normative text in
`SPEC-rebuild-and-rotation.md` §3.2 + a deterministic fixture test for the
`Pending`/`Missing` transition.

---

### EPIC 2 — Notification-never-truth under loss

**Goal.** Demonstrate that convergence depends only on the CAS, never on
notification delivery, by injecting real notification loss.

**Stories.**

- [ ] 2.1 Split traffic into two Nym channels: *entries* (CAS content) and
  *notifications* ("head changed to H"). Notifications are the only thing that
  triggers a pull.
- [ ] 2.2 Drop ≥30% of notifications at the sender (deterministic PRNG,
  seeded, logged) on top of whatever the network drops.
- [ ] 2.3 Measure time-to-convergence per instance as a function of
  notification loss rate (0%, 30%, 60%, 90%).
- [ ] 2.4 Adversarial variant: deliver a notification whose head hash refers
  to an entry that never arrives. Assert the instance does not produce a view
  and does not treat the notification as evidence of anything.
- [ ] 2.5 Identify any code path where a notification's *content* (beyond the
  head hash) influenced state. Any such path is a finding.

**Acceptance.** All instances converge at every loss rate; the convergence-time
curve is recorded. Story 2.5 finds nothing, or what it finds is written up.

**Feeds.** Spike #6 → the mechanical guard is "soak test injects loss and
measures convergence; any notification-content dependency is a compile-time
error via a newtype that carries only `ContentHash`."

---

### EPIC 3 — Axiom 4 and rotation under real time

**Goal.** Put a real clock behind "revocation within one TTL window" and make
concurrent rotation happen naturally.

**Stories.**

- [ ] 3.1 Issue a revocation (as a superseding entry, per `SPEC-cas-and-sigsuite.md`
  §1.6) at wall-clock T on one instance. Record the wall-clock time at which
  each other instance's fold first honours it.
- [ ] 3.2 Repeat across 50 trials at varying times of day. Report the
  distribution of (honour_time − T). Compare to the current TTL constant.
- [ ] 3.3 If any trial exceeds one TTL: either re-size the TTL with the
  distribution as rationale, or record why the axiom is stated as it is and
  what the user-visible consequence is.
- [ ] 3.4 Concurrent rotation: two authors each publish a `GroupRotation`
  within the mixnet's latency window, then merge. Assert `RotationMismatch`
  surfaces as a typed error on every instance and never as a silent view.
- [ ] 3.5 Prototype an explicit fork-resolution entry — a governance entry
  signed by the *pre-fork* group that names the surviving branch — and confirm
  `fold_verified` accepts it under the (a)→(b) ordering rule.

**Acceptance.** Latency distribution recorded. `RotationMismatch` observed
in the wild and resolved via a prototype entry that passes the verified fold.

**Feeds.** Spike #5 → first-class fork-resolution flow in
`SPEC-rebuild-and-rotation.md` §3.7. Axiom 4 → a stated TTL with a transport
assumption, in `ARCHITECTURE.md` §2.

---

### EPIC 4 — Anonymous-sender safety and entry size

**Goal.** Determine which FFP messages can be sent from an anonymous sender
without the payload leaking what the transport hid, and establish an entry
size bound.

**Stories.**

- [ ] 4.1 Enumerate current FFP message shapes (contract offer, acceptance,
  revocation, head notification, entry sync, subscription request). For each,
  list every identity-bearing field (DID, `GroupId`, signature, `prev` link).
- [ ] 4.2 Send each shape via a SURB-based anonymous reply. Classify:
  **anonymous-safe** (no field identifies the sender beyond what the
  application intends to disclose), **pseudonymous** (identifies a long-lived
  key but not a network endpoint), **not anonymous-safe** (identity is
  structurally required by the message's meaning).
- [ ] 4.3 For the Tony Baker pattern specifically: subscriber → artist
  subscription request over a SURB. Confirm the artist learns neither network
  identity nor location, and record exactly what it *does* learn.
- [ ] 4.4 Measure Sphinx fragmentation per message shape. Identify any shape
  whose typical instance exceeds one packet.
- [ ] 4.5 Propose either a hard entry-size bound (with `canonical` module
  enforcement) or a minimal CAS chunking scheme (`Link<Chunk>` list, BLAKE3
  tree-hash-compatible). Recommend one with rationale.

**Acceptance.** A classification table for every FFP message shape. A size
finding with a concrete recommendation.

**Feeds.** New spike or normative text: "anonymous-sender safety" as an FFP
property per message type (`ARCHITECTURE.md` §5.2). CAS chunking:
`SPEC-cas-and-sigsuite.md` §1.7 either gains a chunking section or the
deferral gets a stated size bound.

---

### EPIC 5 — Write-up and teardown

**Goal.** Convert every finding into a spec change or a registered spike, then
delete the kata.

**Stories.**

- [ ] 5.1 `FINDINGS.md`: one section per EPIC, each finding tagged with its
  destination (spec section or spike number).
- [ ] 5.2 For every spec-bound finding: draft the normative text and the
  *deterministic* fixture test that pins it. No Nym dependency in the test.
- [ ] 5.3 For every spike-bound finding: `ARCHITECTURE.md` §12 entry with
  trigger condition.
- [ ] 5.4 Update `ARCHITECTURE.md` §10 (kata series) with a one-paragraph
  summary and the spikes resolved.
- [ ] 5.5 Delete `kata-nym-wild`. Retain `FINDINGS.md` and `PROVENANCE.md` in
  the docs tree.

**Acceptance.** No open finding without a destination. The crate is gone.

---

## Appendix A — Explicitly out of scope

- Adopting Nym as an FFP transport. Separate decision; see §3.
- Evaluating Nym's anonymity guarantees.
- Nym's dVPN / Fast mode (WireGuard) — not exposed via SDK and not a mixnet.
- zk-nym credential provisioning.
- Any change to `folkcore`'s dependency graph.

## Appendix B — Claims marked [verify]

Each must be confirmed against source in EPIC 0, story 0.1, before any other
story begins.

1. SDK usage is free for development and testing (Nym network docs).
2. Rust SDK crates.io publication is paused; Git import required.
3. License of the `nym-sdk` crate itself (as distinct from the monorepo).
4. Raw Mixnet mode provides no ordering guarantee (Nym Rust SDK docs).
5. Browser/WASM path reaches the network only via a gateway over WebSockets.
