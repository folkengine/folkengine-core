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
