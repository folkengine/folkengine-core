//! `validate` catches every way a shell-loaded state can be broken, and
//! `apply` never produces a state that fails it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

mod common;

use std::collections::BTreeSet;

use common::*;
use folkengine::{Action, ActorId, Binding, Defect, Folksonomy, Tag, TagId, Visibility};

fn raw_tag(id: u64, label: &str, parents: &[u64]) -> Tag {
    Tag {
        id: TagId(id),
        label: label.into(),
        aliases: BTreeSet::new(),
        parents: parents.iter().map(|p| TagId(*p)).collect(),
    }
}

#[test]
fn detects_every_defect() {
    let s = Folksonomy::from_parts(
        2, // behind: tags go up to 3
        [
            raw_tag(1, "a", &[2]),
            raw_tag(2, "b", &[1]),  // 1 <-> 2 cycle
            raw_tag(3, "a", &[42]), // duplicate label + dangling parent
        ],
        [Binding {
            item: item("i"),
            tag: TagId(7),
            tagger: ActorId::from("x"),
            visibility: Visibility::Public,
        }],
        [],
    );
    let defects = s.validate();
    assert!(defects.contains(&Defect::DanglingParent {
        child: TagId(3),
        parent: TagId(42)
    }));
    assert!(defects.contains(&Defect::Cycle(vec![TagId(1), TagId(2)])));
    assert!(defects.contains(&Defect::DuplicateLabel("a".into())));
    assert!(defects
        .iter()
        .any(|d| matches!(d, Defect::DanglingBinding(_))));
    assert!(defects.contains(&Defect::IdCounterBehind));
    assert_eq!(defects.len(), 5);
}

#[test]
fn every_transition_preserves_validity() {
    let d = diamond();
    let mut s = tag(&d.s, "a", "x", d.rust, Visibility::Public);
    let (s2, other) = define(&s, "a", "other", &[d.languages]);
    s = s2;
    let script = [
        Action::AddAlias {
            tag: d.rust,
            alias: "rs".into(),
        },
        Action::AddParent {
            child: other,
            parent: d.systems,
        },
        Action::RenameTag {
            tag: other,
            label: "renamed".into(),
        },
        Action::RemoveParent {
            child: other,
            parent: d.languages,
        },
        Action::Tag {
            item: item("y"),
            tag: other,
            visibility: Visibility::Private,
        },
        Action::MergeTags {
            source: other,
            into: d.rust,
        },
        Action::Untag {
            item: item("y"),
            tag: d.rust,
            tagger: None,
        },
        Action::RemoveAlias {
            tag: d.rust,
            alias: "renamed".into(),
        },
    ];
    for a in script {
        let (next, _) = step(&s, "a", a);
        assert!(
            next.validate().is_empty(),
            "state after {next:?} has defects"
        );
        s = next;
    }
    assert!(s.tag(other).is_none());
    assert_eq!(s.resolve("rs"), Some(d.rust));
}

#[test]
fn replay_is_deterministic() {
    // Same action log from the same start → identical state and ids.
    let log = |s: &Folksonomy| {
        let (s, a) = define(s, "u", "A", &[]);
        let (s, b) = define(&s, "u", "B", &[a]);
        let s = tag(&s, "u", "i", b, Visibility::Public);
        (s, a, b)
    };
    let (s1, a1, b1) = log(&open());
    let (s2, a2, b2) = log(&open());
    assert_eq!(s1, s2);
    assert_eq!((a1, b1), (a2, b2));
    assert_eq!(s1.next_tag_id(), TagId(3));
}
