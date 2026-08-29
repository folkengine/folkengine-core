#![allow(dead_code)]

use folkengine::{Action, ActorId, Event, FolkError, Folksonomy, ItemId, TagId, Visibility};

pub fn actor(s: &str) -> ActorId {
    ActorId::from(s)
}

pub fn item(s: &str) -> ItemId {
    ItemId::from(s)
}

/// Apply and unwrap, returning the next state and events.
pub fn step(s: &Folksonomy, who: &str, a: Action) -> (Folksonomy, Vec<Event>) {
    let t = s.apply(&actor(who), a).expect("action should succeed");
    (t.state, t.events)
}

/// Apply and expect an error.
pub fn fails(s: &Folksonomy, who: &str, a: Action) -> FolkError {
    s.apply(&actor(who), a).expect_err("action should fail")
}

/// Define a tag and return the new state and its id.
pub fn define(s: &Folksonomy, who: &str, label: &str, parents: &[TagId]) -> (Folksonomy, TagId) {
    let (s, ev) = step(
        s,
        who,
        Action::DefineTag {
            label: label.to_owned(),
            parents: parents.to_vec(),
        },
    );
    match &ev[..] {
        [Event::TagDefined { tag, .. }] => (s, *tag),
        other => panic!("expected one TagDefined, got {other:?}"),
    }
}

pub fn tag(s: &Folksonomy, who: &str, it: &str, t: TagId, v: Visibility) -> Folksonomy {
    step(
        s,
        who,
        Action::Tag {
            item: item(it),
            tag: t,
            visibility: v,
        },
    )
    .0
}

pub fn open() -> Folksonomy {
    Folksonomy::empty(Vec::<ActorId>::new())
}

pub fn curated(curators: &[&str]) -> Folksonomy {
    Folksonomy::empty(curators.iter().copied())
}

/// The diamond: languages ← rust → systems; rust has two parents.
pub struct Diamond {
    pub s: Folksonomy,
    pub languages: TagId,
    pub systems: TagId,
    pub rust: TagId,
}

pub fn diamond() -> Diamond {
    let s = open();
    let (s, languages) = define(&s, "alice", "Languages", &[]);
    let (s, systems) = define(&s, "alice", "Systems Programming", &[]);
    let (s, rust) = define(&s, "alice", "Rust", &[languages, systems]);
    Diamond {
        s,
        languages,
        systems,
        rust,
    }
}

// ---------------------------------------------------------------------------
// Seeded generation.
//
// `rand` is a banned crate — deterministic replay is the whole point, so the
// generator is part of the contract rather than a dependency whose stream may
// change under us. Every failure here reproduces from a `(seed, step)` pair.
// This is the seed of the `folkengine-testkit` crate (EPIC-00 T-13); it lives
// here until that crate exists.
// ---------------------------------------------------------------------------

/// A `SplitMix64` generator: eight lines of arithmetic with no state but a
/// counter, so a seed names a stream for good.
pub struct Rng(u64);

impl Rng {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform-ish in `0..n`. The modulo bias is irrelevant at these sizes.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0, "below(0)");
        usize::try_from(self.next_u64() % n as u64).unwrap_or(0)
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    /// True one time in `n`.
    pub fn one_in(&mut self, n: usize) -> bool {
        self.below(n) == 0
    }
}

/// Deliberately overlapping and partly invalid: collisions drive
/// `DuplicateLabel`, the blank ones drive `InvalidLabel`, and the spacing and
/// case variants drive the normalization rule.
pub const LABELS: [&str; 10] = [
    "Rust",
    "rust",
    "  Systems   Programming ",
    "Languages",
    "LANGUAGES",
    "Ada",
    "Go",
    "Élan",
    "",
    " \t ",
];

pub const ITEMS: [&str; 4] = ["main.rs", "notes.md", "a", "b"];

/// In a `curated(&["alice"])` state, alice is the curator and the others are
/// not — so `NotAuthorized` is reachable without special-casing.
pub const ACTORS: [&str; 3] = ["alice", "bob", "carol"];

/// An id that is very unlikely to exist, for driving `UnknownTag`.
const NO_SUCH_TAG: TagId = TagId(9_999);

/// An existing tag most of the time, a missing one sometimes.
fn some_tag(rng: &mut Rng, ids: &[TagId]) -> TagId {
    if ids.is_empty() || rng.one_in(8) {
        NO_SUCH_TAG
    } else {
        *rng.pick(ids)
    }
}

fn some_visibility(rng: &mut Rng) -> Visibility {
    if rng.one_in(2) {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

/// A random action against `s`. Weighted toward `DefineTag` and `Tag` so that
/// traces grow states worth testing rather than bouncing off an empty one.
pub fn random_action(rng: &mut Rng, s: &Folksonomy) -> Action {
    let ids: Vec<TagId> = s.tags().map(|t| t.id).collect();
    match rng.below(16) {
        0..=2 => Action::DefineTag {
            label: (*rng.pick(&LABELS)).to_owned(),
            parents: {
                let n = rng.below(3);
                (0..n).map(|_| some_tag(rng, &ids)).collect()
            },
        },
        3 => Action::RenameTag {
            tag: some_tag(rng, &ids),
            label: (*rng.pick(&LABELS)).to_owned(),
        },
        4 | 5 => Action::AddAlias {
            tag: some_tag(rng, &ids),
            alias: (*rng.pick(&LABELS)).to_owned(),
        },
        6 => Action::RemoveAlias {
            tag: some_tag(rng, &ids),
            alias: (*rng.pick(&LABELS)).to_owned(),
        },
        7 | 8 => Action::AddParent {
            child: some_tag(rng, &ids),
            parent: some_tag(rng, &ids),
        },
        9 => Action::RemoveParent {
            child: some_tag(rng, &ids),
            parent: some_tag(rng, &ids),
        },
        10 => Action::MergeTags {
            source: some_tag(rng, &ids),
            into: some_tag(rng, &ids),
        },
        11 => Action::RetireTag(some_tag(rng, &ids)),
        12..=14 => Action::Tag {
            item: item(rng.pick(&ITEMS)),
            tag: some_tag(rng, &ids),
            visibility: some_visibility(rng),
        },
        _ => Action::Untag {
            item: item(rng.pick(&ITEMS)),
            tag: some_tag(rng, &ids),
            tagger: if rng.one_in(2) {
                None
            } else {
                Some(actor(rng.pick(&ACTORS)))
            },
        },
    }
}

/// The name of an action's arm, for coverage accounting.
#[must_use]
pub fn arm_of(a: &Action) -> &'static str {
    match a {
        Action::DefineTag { .. } => "DefineTag",
        Action::RenameTag { .. } => "RenameTag",
        Action::AddAlias { .. } => "AddAlias",
        Action::RemoveAlias { .. } => "RemoveAlias",
        Action::AddParent { .. } => "AddParent",
        Action::RemoveParent { .. } => "RemoveParent",
        Action::MergeTags { .. } => "MergeTags",
        Action::RetireTag(_) => "RetireTag",
        Action::Tag { .. } => "Tag",
        Action::Untag { .. } => "Untag",
    }
}

/// Every arm `arm_of` can return. Asserting against this list is what makes
/// "covers every action arm" a test rather than a hope.
pub const ARMS: [&str; 10] = [
    "DefineTag",
    "RenameTag",
    "AddAlias",
    "RemoveAlias",
    "AddParent",
    "RemoveParent",
    "MergeTags",
    "RetireTag",
    "Tag",
    "Untag",
];
