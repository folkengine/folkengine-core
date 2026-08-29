//! Every `FolkError` arm is reachable — state coverage over the failure space.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

mod common;

use common::*;
use folkengine::{Action, FolkError, Selection, TagId, TagRef, Visibility};

#[test]
fn unknown_tag() {
    let s = open();
    let ghost = TagId(999);
    assert_eq!(
        fails(&s, "a", Action::RetireTag(ghost)),
        FolkError::UnknownTag(ghost)
    );
    assert_eq!(
        fails(
            &s,
            "a",
            Action::DefineTag {
                label: "x".into(),
                parents: vec![ghost]
            }
        ),
        FolkError::UnknownTag(ghost)
    );
    assert_eq!(
        fails(
            &s,
            "a",
            Action::Tag {
                item: item("i"),
                tag: ghost,
                visibility: Visibility::Public
            }
        ),
        FolkError::UnknownTag(ghost)
    );
    assert_eq!(s.ancestors(ghost), Err(FolkError::UnknownTag(ghost)));
    assert_eq!(
        s.query(&actor("a"), &Selection::all_of([TagRef::exact(ghost)])),
        Err(FolkError::UnknownTag(ghost))
    );
    assert_eq!(
        s.query(
            &actor("a"),
            &Selection::everything().but_not(TagRef::under(ghost))
        ),
        Err(FolkError::UnknownTag(ghost))
    );
}

#[test]
fn invalid_label() {
    let s = open();
    assert_eq!(
        fails(
            &s,
            "a",
            Action::DefineTag {
                label: " \t\n".into(),
                parents: vec![]
            }
        ),
        FolkError::InvalidLabel(" \t\n".into())
    );
    let (s, t) = define(&s, "a", "ok", &[]);
    assert_eq!(
        fails(
            &s,
            "a",
            Action::RenameTag {
                tag: t,
                label: String::new()
            }
        ),
        FolkError::InvalidLabel(String::new())
    );
    assert_eq!(
        fails(
            &s,
            "a",
            Action::AddAlias {
                tag: t,
                alias: "  ".into()
            }
        ),
        FolkError::InvalidLabel("  ".into())
    );
}

#[test]
fn duplicate_label_across_labels_and_aliases() {
    let s = open();
    let (s, a) = define(&s, "a", "alpha", &[]);
    let (s, b) = define(&s, "a", "beta", &[]);
    let (s, _) = step(
        &s,
        "a",
        Action::AddAlias {
            tag: a,
            alias: "first".into(),
        },
    );
    assert_eq!(
        fails(
            &s,
            "a",
            Action::DefineTag {
                label: "First".into(),
                parents: vec![]
            }
        ),
        FolkError::DuplicateLabel(a)
    );
    assert_eq!(
        fails(
            &s,
            "a",
            Action::RenameTag {
                tag: b,
                label: "alpha".into()
            }
        ),
        FolkError::DuplicateLabel(a)
    );
    assert_eq!(
        fails(
            &s,
            "a",
            Action::AddAlias {
                tag: b,
                alias: "first".into()
            }
        ),
        FolkError::DuplicateLabel(a)
    );
    // A tag may take its own alias as its label (promotion), and re-adding
    // its own label as an alias is a silent no-op, not a duplicate.
    let (s, ev) = step(
        &s,
        "a",
        Action::RenameTag {
            tag: a,
            label: "first".into(),
        },
    );
    assert_eq!(ev.len(), 1);
    assert!(!s.tag(a).unwrap().aliases.contains("first"));
    let (_, ev) = step(
        &s,
        "a",
        Action::AddAlias {
            tag: a,
            alias: "first".into(),
        },
    );
    assert!(ev.is_empty());
}

#[test]
fn would_cycle_on_add_parent_self_edge_and_merge() {
    let d = diamond();
    assert_eq!(
        fails(
            &d.s,
            "a",
            Action::AddParent {
                child: d.languages,
                parent: d.rust
            }
        ),
        FolkError::WouldCycle
    );
    assert_eq!(
        fails(
            &d.s,
            "a",
            Action::AddParent {
                child: d.rust,
                parent: d.rust
            }
        ),
        FolkError::WouldCycle
    );
    // Deeper: root -> mid -> leaf; root under leaf closes a 3-cycle.
    let s = open();
    let (s, root) = define(&s, "a", "root", &[]);
    let (s, mid) = define(&s, "a", "mid", &[root]);
    let (s, leaf) = define(&s, "a", "leaf", &[mid]);
    assert_eq!(
        fails(
            &s,
            "a",
            Action::AddParent {
                child: root,
                parent: leaf
            }
        ),
        FolkError::WouldCycle
    );
    // Merging a tag into its own descendant would make the descendant its
    // own ancestor: mid's parent root … merge root into leaf → leaf gets
    // root's children (mid) re-parented to leaf, and mid is leaf's parent.
    assert_eq!(
        fails(
            &s,
            "a",
            Action::MergeTags {
                source: root,
                into: leaf
            }
        ),
        FolkError::WouldCycle
    );
    // Nothing leaked from the rejected merge.
    assert!(s.tag(root).is_some());
    assert!(s.validate().is_empty());
}

#[test]
fn not_authorized() {
    let s = curated(&["cur"]);
    let (s, t) = define(&s, "folk", "t", &[]);
    assert_eq!(
        fails(
            &s,
            "folk",
            Action::AddParent {
                child: t,
                parent: t
            }
        ),
        FolkError::NotAuthorized
    );
    assert_eq!(
        fails(&s, "folk", Action::MergeTags { source: t, into: t }),
        FolkError::NotAuthorized
    );
}

#[test]
fn tag_in_use() {
    let d = diamond();
    assert_eq!(
        fails(&d.s, "a", Action::RetireTag(d.systems)),
        FolkError::TagInUse(d.systems)
    );
}

#[test]
fn self_merge() {
    let d = diamond();
    assert_eq!(
        fails(
            &d.s,
            "a",
            Action::MergeTags {
                source: d.rust,
                into: d.rust
            }
        ),
        FolkError::SelfMerge
    );
}

#[test]
fn not_bound() {
    let d = diamond();
    assert_eq!(
        fails(
            &d.s,
            "a",
            Action::Untag {
                item: item("nothing"),
                tag: d.rust,
                tagger: None
            }
        ),
        FolkError::NotBound
    );
    let s = tag(&d.s, "b", "x", d.rust, Visibility::Public);
    // Your own binding on an item someone else tagged still doesn't exist.
    assert_eq!(
        fails(
            &s,
            "a",
            Action::Untag {
                item: item("x"),
                tag: d.rust,
                tagger: None
            }
        ),
        FolkError::NotBound
    );
}

#[test]
fn not_present() {
    let d = diamond();
    assert_eq!(
        fails(
            &d.s,
            "a",
            Action::RemoveParent {
                child: d.languages,
                parent: d.rust
            }
        ),
        FolkError::NotPresent
    );
    assert_eq!(
        fails(
            &d.s,
            "a",
            Action::RemoveAlias {
                tag: d.rust,
                alias: "nope".into()
            }
        ),
        FolkError::NotPresent
    );
}

#[test]
fn rejection_never_mutates() {
    let d = diamond();
    let before = d.s.clone();
    let _ = d.s.apply(
        &actor("a"),
        Action::AddParent {
            child: d.languages,
            parent: d.rust,
        },
    );
    let _ = d.s.apply(&actor("a"), Action::RetireTag(d.languages));
    assert_eq!(d.s, before);
}
