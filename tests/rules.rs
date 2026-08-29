//! The seven named discretionary rules from the charter, pinned.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

mod common;

use common::*;
use folkengine::{
    normalize_label, Action, Event, FolkError, Selection, TagCount, TagRef, Visibility,
};

// Rule 1 — label normalization: trim, single-space, Unicode lowercase, no NFC.
#[test]
fn rule1_normalization_collapses_case_and_whitespace_but_not_unicode_forms() {
    assert_eq!(normalize_label("  Rust "), "rust");
    assert_eq!(
        normalize_label("Systems\t\tProgramming\n"),
        "systems programming"
    );
    assert_eq!(normalize_label("ÉCOLE"), "école");
    // Composed vs decomposed é remain distinct: no NFC.
    assert_ne!(normalize_label("\u{e9}"), normalize_label("e\u{301}"));

    let s = open();
    let (s, rust) = define(&s, "a", "  RUST  ", &[]);
    assert_eq!(s.tag(rust).unwrap().label, "rust");
    assert_eq!(s.resolve("Rust"), Some(rust));
    assert_eq!(
        fails(
            &s,
            "a",
            Action::DefineTag {
                label: " rust".into(),
                parents: vec![]
            }
        ),
        FolkError::DuplicateLabel(rust)
    );
}

// Rule 2 — retire requires empty.
#[test]
fn rule2_retire_refuses_tags_with_bindings_or_children() {
    let d = diamond();
    assert_eq!(
        fails(&d.s, "a", Action::RetireTag(d.languages)),
        FolkError::TagInUse(d.languages)
    );
    let s = tag(&d.s, "a", "x", d.rust, Visibility::Public);
    assert_eq!(
        fails(&s, "a", Action::RetireTag(d.rust)),
        FolkError::TagInUse(d.rust)
    );
    // Leaf with nothing bound retires.
    let (s, ev) = step(&d.s, "a", Action::RetireTag(d.rust));
    assert_eq!(
        ev,
        vec![Event::TagRetired {
            tag: d.rust,
            label: "rust".into(),
            by: actor("a")
        }]
    );
    assert!(s.tag(d.rust).is_none());
    let (_, ev) = step(&s, "a", Action::RetireTag(d.languages));
    assert!(matches!(ev[..], [Event::TagRetired { .. }]));
}

// Rule 3 — merge semantics.
#[test]
fn rule3_merge_moves_bindings_absorbs_labels_rewires_edges_and_orders_events() {
    let s = open();
    let (s, root) = define(&s, "a", "root", &[]);
    let (s, into) = define(&s, "a", "into", &[root]);
    let (s, extra) = define(&s, "a", "extra", &[]);
    let (s, source) = define(&s, "a", "source", &[extra]);
    let (s, child) = define(&s, "a", "child", &[source]);
    let (s, _) = step(
        &s,
        "a",
        Action::AddAlias {
            tag: source,
            alias: "src".into(),
        },
    );
    let s = tag(&s, "a", "moves", source, Visibility::Private);
    let s = tag(&s, "b", "dup", source, Visibility::Public);
    let s = tag(&s, "b", "dup", into, Visibility::Public); // already on target → dropped

    let (s, ev) = step(&s, "a", Action::MergeTags { source, into });

    // Structure.
    assert!(s.tag(source).is_none());
    let t = s.tag(into).unwrap();
    assert!(t.parents.contains(&root) && t.parents.contains(&extra));
    assert!(t.aliases.contains("source") && t.aliases.contains("src"));
    assert!(s.tag(child).unwrap().parents.contains(&into));
    assert!(!s.tag(child).unwrap().parents.contains(&source));
    assert_eq!(s.resolve("src"), Some(into));
    assert_eq!(s.resolve("source"), Some(into));

    // Bindings: "moves" moved with its visibility; "dup" dropped.
    let bindings: Vec<_> = s.bindings().collect();
    assert_eq!(bindings.len(), 2);
    assert!(bindings.iter().all(|b| b.tag == into));
    assert!(bindings
        .iter()
        .any(|b| b.item == item("moves") && b.visibility == Visibility::Private));

    // Event order: edges, aliases, bindings, summary last.
    let kinds: Vec<&str> = ev
        .iter()
        .map(|e| match e {
            Event::ParentRemoved { .. } => "parent-removed",
            Event::ParentAdded { .. } => "parent-added",
            Event::AliasAdded { .. } => "alias-added",
            Event::Untagged { .. } => "untagged",
            Event::Tagged { .. } => "tagged",
            Event::TagsMerged { .. } => "tags-merged",
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            "parent-removed", // child -x source
            "parent-added",   // child -> into
            "parent-added",   // into -> extra
            "alias-added",    // "source"
            "alias-added",    // "src"
            "untagged",       // dup by b — dropped, no tagged follows
            "untagged",       // moves by a
            "tagged",         // moves by a, now on `into`
            "tags-merged",
        ]
    );
    assert_eq!(kinds.first(), Some(&"parent-removed"));
    assert_eq!(kinds.last(), Some(&"tags-merged"));
    let last_edge = kinds.iter().rposition(|k| k.starts_with("parent")).unwrap();
    let first_alias = kinds.iter().position(|k| *k == "alias-added").unwrap();
    let last_alias = kinds.iter().rposition(|k| *k == "alias-added").unwrap();
    let first_binding = kinds.iter().position(|k| *k == "untagged").unwrap();
    assert!(last_edge < first_alias && last_alias < first_binding);
    assert!(matches!(
        ev.last(),
        Some(Event::TagsMerged {
            moved_bindings: 1,
            ..
        })
    ));
    assert!(s.validate().is_empty());
}

// Rule 4 — idempotent tagging.
#[test]
fn rule4_identical_binding_emits_nothing_and_visibility_change_emits_one_event() {
    let d = diamond();
    let a = Action::Tag {
        item: item("x"),
        tag: d.rust,
        visibility: Visibility::Public,
    };
    let (s, ev) = step(&d.s, "a", a.clone());
    assert!(matches!(ev[..], [Event::Tagged { .. }]));
    let (s2, ev) = step(&s, "a", a);
    assert!(ev.is_empty());
    assert_eq!(s, s2);
    let (_, ev) = step(
        &s,
        "a",
        Action::Tag {
            item: item("x"),
            tag: d.rust,
            visibility: Visibility::Private,
        },
    );
    assert_eq!(
        ev,
        vec![Event::VisibilityChanged {
            item: item("x"),
            tag: d.rust,
            tagger: actor("a"),
            visibility: Visibility::Private
        }]
    );
}

// Rule 5 — governance split.
#[test]
fn rule5_curation_gates_structure_but_never_defining_or_tagging() {
    let s = curated(&["cur"]);
    let (s, t) = define(&s, "folk", "anything", &[]);
    let s = tag(&s, "folk", "x", t, Visibility::Public);
    assert_eq!(
        fails(
            &s,
            "folk",
            Action::RenameTag {
                tag: t,
                label: "b".into()
            }
        ),
        FolkError::NotAuthorized
    );
    assert_eq!(
        fails(
            &s,
            "folk",
            Action::AddAlias {
                tag: t,
                alias: "b".into()
            }
        ),
        FolkError::NotAuthorized
    );
    assert_eq!(
        fails(&s, "folk", Action::RetireTag(t)),
        FolkError::NotAuthorized
    );
    let (s, _) = step(
        &s,
        "cur",
        Action::RenameTag {
            tag: t,
            label: "b".into(),
        },
    );
    // Untag: own is open; another's needs a curator.
    assert_eq!(
        fails(
            &s,
            "other",
            Action::Untag {
                item: item("x"),
                tag: t,
                tagger: Some(actor("folk"))
            }
        ),
        FolkError::NotAuthorized
    );
    let (_, ev) = step(
        &s,
        "cur",
        Action::Untag {
            item: item("x"),
            tag: t,
            tagger: Some(actor("folk")),
        },
    );
    assert!(matches!(ev[..], [Event::Untagged { .. }]));
    let (_, ev) = step(
        &s,
        "folk",
        Action::Untag {
            item: item("x"),
            tag: t,
            tagger: None,
        },
    );
    assert!(matches!(ev[..], [Event::Untagged { .. }]));

    // Open vocabulary: everyone edits structure, nobody removes others' bindings.
    let o = open();
    let (o, t) = define(&o, "p", "t", &[]);
    let o = tag(&o, "p", "x", t, Visibility::Public);
    step(
        &o,
        "q",
        Action::RenameTag {
            tag: t,
            label: "u".into(),
        },
    );
    assert_eq!(
        fails(
            &o,
            "q",
            Action::Untag {
                item: item("x"),
                tag: t,
                tagger: Some(actor("p"))
            }
        ),
        FolkError::NotAuthorized
    );
}

// Rule 6 — result orderings.
#[test]
fn rule6_items_sort_bytewise_and_facets_by_count_desc_then_id() {
    let d = diamond();
    let (s, other) = define(&d.s, "a", "other", &[]);
    let s = tag(&s, "a", "b", d.rust, Visibility::Public);
    let s = tag(&s, "a", "a", d.rust, Visibility::Public);
    let s = tag(&s, "a", "B", d.rust, Visibility::Public);
    let s = tag(&s, "a", "a", other, Visibility::Public);
    let s = tag(&s, "a", "b", d.languages, Visibility::Public);
    let s = tag(&s, "a", "B", d.languages, Visibility::Public);
    let s = tag(&s, "a", "a", d.languages, Visibility::Public);
    let items = s.query(&actor("a"), &Selection::everything()).unwrap();
    assert_eq!(items, vec![item("B"), item("a"), item("b")]); // uppercase first: bytewise
    let f = s.facets(&actor("a"), &Selection::everything()).unwrap();
    assert_eq!(
        f,
        vec![
            TagCount {
                tag: d.languages,
                count: 3
            },
            TagCount {
                tag: d.rust,
                count: 3
            },
            TagCount {
                tag: other,
                count: 1
            },
        ]
    );
    assert!(d.languages < d.rust, "tie broken by id ascending");
}

// Rule 7 — transitive only when asked.
#[test]
fn rule7_hierarchy_consulted_only_for_transitive_refs() {
    let d = diamond();
    let s = tag(&d.s, "a", "main.rs", d.rust, Visibility::Public);
    let a = actor("a");
    assert_eq!(
        s.query(&a, &Selection::all_of([TagRef::under(d.languages)]))
            .unwrap(),
        vec![item("main.rs")]
    );
    assert_eq!(
        s.query(&a, &Selection::all_of([TagRef::exact(d.languages)]))
            .unwrap(),
        Vec::new()
    );
    assert_eq!(
        s.query(&a, &Selection::all_of([TagRef::under(d.systems)]))
            .unwrap(),
        vec![item("main.rs")]
    );
    // Exclusion is transitive on request too.
    assert_eq!(
        s.query(
            &a,
            &Selection::everything().but_not(TagRef::under(d.languages))
        )
        .unwrap(),
        Vec::new()
    );
    // Facets count direct bindings only.
    let f = s
        .facets(&a, &Selection::all_of([TagRef::under(d.languages)]))
        .unwrap();
    assert_eq!(
        f,
        vec![TagCount {
            tag: d.rust,
            count: 1
        }]
    );
}
