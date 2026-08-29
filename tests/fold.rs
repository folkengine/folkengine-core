//! The transition contract: `apply` returns a state and events, and the two
//! halves must be the same fact told twice.
//!
//! This is the crate-level form of `rebuild_is_deterministic_and_idempotent`
//! (`SPEC-rebuild-and-rotation.md` §3.6). Without it the state path and the
//! event path can drift silently, and a shell that rebuilds an index from the
//! event stream would get something other than the state the kernel computed —
//! which is exactly the substrate's disposable-index invariant being false.
//!
//! Everything here is seeded. A failure reproduces from the `(seed, step)`
//! pair in its message; nothing consults entropy or a clock.

use std::collections::BTreeMap;

use folkengine::{Action, ActorId, Event, Folksonomy, ItemId, TagId, Visibility};

mod common;
use common::{
    actor, arm_of, curated, define, item, open, random_action, Rng, ACTORS, ARMS, LABELS,
};

/// Traces per configuration. Enough that every arm is exercised both ways;
/// the coverage assertion at the end of `fold_reproduces_every_transition` is
/// what fails if this is ever lowered too far.
const SEEDS: u64 = 400;
/// Actions per trace.
const STEPS: usize = 40;

/// How many times each action arm was accepted and rejected across the run.
#[derive(Default)]
struct Coverage(BTreeMap<&'static str, (usize, usize)>);

impl Coverage {
    fn accepted(&mut self, arm: &'static str) {
        self.0.entry(arm).or_default().0 += 1;
    }

    fn rejected(&mut self, arm: &'static str) {
        self.0.entry(arm).or_default().1 += 1;
    }

    /// Every arm must have been both taken and refused. An arm that was never
    /// accepted proves nothing about `fold`; an arm that was never rejected
    /// leaves the "rejection changes nothing" half untested.
    fn assert_complete(&self) {
        let missing: Vec<String> = ARMS
            .iter()
            .map(|arm| (arm, self.0.get(arm).copied().unwrap_or((0, 0))))
            .filter(|(_, (ok, err))| *ok == 0 || *err == 0)
            .map(|(arm, (ok, err))| format!("{arm}: {ok} accepted, {err} rejected"))
            .collect();
        assert!(
            missing.is_empty(),
            "generator no longer reaches every action arm both ways:\n  {}",
            missing.join("\n  ")
        );
    }
}

/// The headline property, in both of its forms.
///
/// Per step: `pre.fold(events) == post`.
/// Per trace: `genesis.fold(every event in order) == final state` — the
/// `fold(empty, events) == next_state` of the epic's definition of done.
#[test]
fn fold_reproduces_every_transition() {
    let mut coverage = Coverage::default();

    for seed in 0..SEEDS {
        // Alternate open and curated so `NotAuthorized` is on the path and the
        // curated arms are exercised by a non-curator as well as a curator.
        let genesis = if seed % 2 == 0 {
            open()
        } else {
            curated(&["alice"])
        };
        let mut s = genesis.clone();
        let mut trace: Vec<Event> = Vec::new();
        let mut rng = Rng::new(seed);

        for step in 0..STEPS {
            let who = actor(rng.pick(&ACTORS));
            let action = random_action(&mut rng, &s);
            let arm = arm_of(&action);
            let before = s.clone();

            match s.apply(&who, action) {
                Ok(t) => {
                    assert_eq!(
                        s.fold(&t.events),
                        t.state,
                        "fold(pre, events) != post at seed {seed} step {step} arm {arm}"
                    );
                    coverage.accepted(arm);
                    trace.extend(t.events);
                    s = t.state;
                }
                Err(e) => {
                    // A rejection emits no events and mutates nothing.
                    assert_eq!(
                        s, before,
                        "rejection ({e}) mutated the state at seed {seed} step {step} arm {arm}"
                    );
                    coverage.rejected(arm);
                }
            }
        }

        // The whole trace, folded once from genesis.
        assert_eq!(
            genesis.fold(&trace),
            s,
            "fold(genesis, whole trace) != final state at seed {seed}"
        );
        // And the state the trace built is a state the kernel would accept.
        assert_eq!(
            s.validate(),
            vec![],
            "seed {seed} built a state with defects"
        );
    }

    coverage.assert_complete();
}

/// Folding is a function of the events alone: the same trace folded twice, and
/// folded in one pass or two, lands in the same place. This is the idempotence
/// half of §3.6.
#[test]
fn folding_is_deterministic_and_splittable() {
    for seed in 0..64 {
        let genesis = open();
        let mut s = genesis.clone();
        let mut trace: Vec<Event> = Vec::new();
        let mut rng = Rng::new(seed ^ 0xF01D);

        for _ in 0..STEPS {
            let who = actor(rng.pick(&ACTORS));
            let action = random_action(&mut rng, &s);
            if let Ok(t) = s.apply(&who, action) {
                trace.extend(t.events);
                s = t.state;
            }
        }

        assert_eq!(genesis.fold(&trace), genesis.fold(&trace), "seed {seed}");
        for cut in 0..=trace.len() {
            let (head, tail) = trace.split_at(cut);
            assert_eq!(
                genesis.fold(head).fold(tail),
                s,
                "split fold at {cut} diverged, seed {seed}"
            );
        }
    }
}

/// `fold` carries curators through untouched, because no event changes them.
/// This is the honest statement of the gap EPIC-00 T-06 closes: until curator
/// membership is an action, a curator change is invisible to replay.
#[test]
fn fold_carries_curators_through_unchanged() {
    let s = curated(&["alice", "bob"]);
    let (s2, rust) = define(&s, "alice", "Rust", &[]);
    let t = s2
        .apply(
            &actor("alice"),
            Action::Tag {
                item: item("main.rs"),
                tag: rust,
                visibility: Visibility::Public,
            },
        )
        .expect("tagging should succeed");

    assert_eq!(s2.fold(&t.events).curators(), s.curators());

    // Folding the same events onto a *differently* curated genesis reproduces
    // everything except the curator set — the gap, stated as a test.
    let other = Folksonomy::empty(["carol"]);
    let mixed = other.fold(&t.events);
    assert_eq!(
        mixed.curators(),
        &[ActorId::from("carol")].into_iter().collect()
    );
}

/// The arm most able to diverge: a merge emits primitive events for every
/// rewired edge, absorbed label and moved binding, and only the trailing
/// `TagsMerged` says the source is gone.
#[test]
fn fold_reproduces_a_merge_that_rewires_edges_labels_and_bindings() {
    let s = open();
    let (s, languages) = define(&s, "alice", "Languages", &[]);
    let (s, systems) = define(&s, "alice", "Systems", &[]);
    // `source` has a parent, a child, an alias, and bindings — one of which
    // duplicates a binding already on `into`, so it is dropped, not counted.
    let (s, source) = define(&s, "alice", "Rustlang", &[languages]);
    let (s, child) = define(&s, "alice", "Tokio", &[source]);
    let (s, into) = define(&s, "alice", "Rust", &[systems]);

    let s = s
        .apply(
            &actor("alice"),
            Action::AddAlias {
                tag: source,
                alias: "Oxide".into(),
            },
        )
        .expect("alias")
        .state;

    let mut s = s;
    for (who, it, tag) in [
        ("alice", "main.rs", source),
        ("bob", "notes.md", source),
        ("alice", "main.rs", into),
    ] {
        s = s
            .apply(
                &actor(who),
                Action::Tag {
                    item: item(it),
                    tag,
                    visibility: Visibility::Public,
                },
            )
            .expect("tag")
            .state;
    }

    let t = s
        .apply(&actor("alice"), Action::MergeTags { source, into })
        .expect("merge should succeed");

    assert_eq!(s.fold(&t.events), t.state);

    // …and the merge really did the work, so the equality above is not two
    // no-ops agreeing with each other.
    assert!(t.state.tag(source).is_none(), "source should be gone");
    let absorbed = t.state.tag(into).expect("into survives");
    assert!(absorbed.aliases.contains("rustlang"));
    assert!(absorbed.aliases.contains("oxide"));
    assert!(absorbed.parents.contains(&languages) && absorbed.parents.contains(&systems));
    assert!(t
        .state
        .tag(child)
        .expect("child survives")
        .parents
        .contains(&into));
    assert!(t.events.iter().any(|e| matches!(
        e,
        Event::TagsMerged {
            moved_bindings: 1,
            ..
        }
    )));
}

/// Renaming a tag to one of its own aliases promotes the alias — it leaves the
/// alias set. `TagRenamed` carries only the labels, so `fold` has to know that
/// rule too, and this is the test that says so.
#[test]
fn fold_reproduces_a_rename_that_promotes_an_alias() {
    let s = open();
    let (s, rust) = define(&s, "alice", "Rustlang", &[]);
    let s = s
        .apply(
            &actor("alice"),
            Action::AddAlias {
                tag: rust,
                alias: "Rust".into(),
            },
        )
        .expect("alias")
        .state;

    let t = s
        .apply(
            &actor("alice"),
            Action::RenameTag {
                tag: rust,
                label: "Rust".into(),
            },
        )
        .expect("rename should succeed");

    assert_eq!(s.fold(&t.events), t.state);
    let after = t.state.tag(rust).expect("tag survives");
    assert_eq!(after.label, "rust");
    assert!(
        !after.aliases.contains("rust"),
        "the promoted alias should have left the alias set"
    );
}

/// Replay must reproduce the id allocator, not just the tags: a state whose
/// `next_tag_id` lags behind its ids is a `Defect::IdCounterBehind`, and the
/// next `DefineTag` would collide.
#[test]
fn fold_reproduces_the_id_allocator() {
    let s = open();
    let mut trace: Vec<Event> = Vec::new();
    let mut s2 = s.clone();
    for label in LABELS.iter().filter(|l| !l.trim().is_empty()).take(4) {
        if let Ok(t) = s2.apply(
            &actor("alice"),
            Action::DefineTag {
                label: (*label).to_owned(),
                parents: vec![],
            },
        ) {
            trace.extend(t.events);
            s2 = t.state;
        }
    }

    let replayed = s.fold(&trace);
    assert_eq!(replayed.next_tag_id(), s2.next_tag_id());
    assert_eq!(replayed.validate(), vec![]);

    // Retiring every tag must not wind the counter back.
    let mut s3 = s2.clone();
    let ids: Vec<TagId> = s3.tags().map(|t| t.id).collect();
    let mut more = trace.clone();
    for id in ids {
        let t = s3
            .apply(&actor("alice"), Action::RetireTag(id))
            .expect("retire");
        more.extend(t.events);
        s3 = t.state;
    }
    assert_eq!(s.fold(&more), s3);
    assert_eq!(s3.next_tag_id(), s2.next_tag_id());
}

/// An `ItemId` the kernel never interprets still has to survive replay
/// bytewise, because binding identity is `(item, tag, tagger)`.
#[test]
fn fold_preserves_opaque_item_bytes() {
    let s = open();
    let (s, rust) = define(&s, "alice", "Rust", &[]);
    let weird = ItemId::from("  MiXeD \u{2028} Case/../path?q=1  ");
    let t = s
        .apply(
            &actor("alice"),
            Action::Tag {
                item: weird.clone(),
                tag: rust,
                visibility: Visibility::Private,
            },
        )
        .expect("tag");

    let replayed = s.fold(&t.events);
    assert_eq!(replayed, t.state);
    assert_eq!(
        replayed.bindings().map(|b| b.item).collect::<Vec<_>>(),
        vec![weird]
    );
}
