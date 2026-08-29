//! Invariant #5: the visibility rule lives in one place and every read obeys it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

mod common;

use common::*;
use folkengine::{Action, Selection, TagCount, TagRef, Visibility};

#[test]
fn private_bindings_are_visible_only_to_their_tagger_everywhere() {
    let d = diamond();
    let s = tag(&d.s, "alice", "shared", d.rust, Visibility::Public);
    let s = tag(&s, "bob", "secret", d.rust, Visibility::Private);
    let s = tag(&s, "bob", "shared", d.languages, Visibility::Private);
    let alice = actor("alice");
    let bob = actor("bob");
    let carol = actor("carol");

    // view_for
    assert_eq!(s.view_for(&alice).bindings.len(), 1);
    assert_eq!(s.view_for(&bob).bindings.len(), 3);
    assert_eq!(s.view_for(&carol).bindings.len(), 1);
    assert_eq!(
        s.view_for(&carol).tags.len(),
        3,
        "the vocabulary itself is public"
    );

    // query
    let everything = Selection::everything();
    assert_eq!(s.query(&alice, &everything).unwrap(), vec![item("shared")]);
    assert_eq!(
        s.query(&bob, &everything).unwrap(),
        vec![item("secret"), item("shared")]
    );

    // facets: bob's private `languages` binding on "shared" only counts for bob
    let f_alice = s.facets(&alice, &everything).unwrap();
    assert_eq!(
        f_alice,
        vec![TagCount {
            tag: d.rust,
            count: 1
        }]
    );
    let f_bob = s.facets(&bob, &everything).unwrap();
    assert_eq!(
        f_bob,
        vec![
            TagCount {
                tag: d.rust,
                count: 2
            },
            TagCount {
                tag: d.languages,
                count: 1
            }
        ]
    );

    // tags_of
    assert_eq!(s.tags_of(&carol, &item("shared")).len(), 1);
    assert_eq!(s.tags_of(&bob, &item("shared")).len(), 2);
    assert!(s.tags_of(&carol, &item("secret")).is_empty());
}

#[test]
fn taggers_filter_narrows_before_evaluation() {
    let d = diamond();
    let s = tag(&d.s, "alice", "x", d.rust, Visibility::Public);
    let s = tag(&s, "bob", "y", d.rust, Visibility::Public);
    let sel = Selection::all_of([TagRef::exact(d.rust)]).by([actor("bob")]);
    assert_eq!(s.query(&actor("carol"), &sel).unwrap(), vec![item("y")]);
}

#[test]
fn a_view_is_not_a_state() {
    // Compile-time property: FolksonomyView has no `apply`. Documented here
    // so the intent survives refactors; the assertion is the type system.
    let d = diamond();
    let v = d.s.view_for(&actor("alice"));
    assert_eq!(v.tags.len(), 3);
    let _ = Action::RetireTag(d.rust); // exists on Folksonomy, not on the view
}
