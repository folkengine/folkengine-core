//! # folkengine — a folksonomy domain kernel
//!
//! The pure logic of a folksonomy: a poly-hierarchical tag vocabulary,
//! `(tagger, item, tag)` bindings, and per-actor selections over them.
//!
//! This crate is a *domain kernel*: every operation is a total function from
//! state and input to new state and events. It performs no I/O, consults no
//! clock, and knows nothing about files, paths, URLs, databases or how it is
//! being called. The language-neutral contract it implements is
//! `wit/folkengine.wit`; the Rust API mirrors it one-to-one.
//!
//! ```
//! use folkengine::{Action, ActorId, Folksonomy, ItemId, Selection, TagRef, Visibility};
//!
//! let alice = ActorId::from("alice");
//! let bob = ActorId::from("bob");
//!
//! // Open vocabulary: no curators.
//! let s = Folksonomy::empty(Vec::<ActorId>::new());
//! let t = s.apply(&alice, Action::DefineTag { label: "Languages".into(), parents: vec![] })?;
//! let languages = t.state.resolve("languages").unwrap();
//! let t = t.state.apply(&alice, Action::DefineTag { label: "Rust".into(), parents: vec![languages] })?;
//! let rust = t.state.resolve("rust").unwrap();
//!
//! let t = t.state.apply(&alice, Action::Tag { item: ItemId::from("main.rs"), tag: rust, visibility: Visibility::Public })?;
//! let t = t.state.apply(&bob, Action::Tag { item: ItemId::from("notes.md"), tag: rust, visibility: Visibility::Private })?;
//! let s = t.state;
//!
//! // A transitive selection on `languages` finds what was tagged `rust`…
//! let under_languages = Selection::all_of([TagRef::under(languages)]);
//! assert_eq!(s.query(&alice, &under_languages)?, vec![ItemId::from("main.rs")]);
//! // …and bob's private binding only when bob is asking.
//! assert_eq!(s.query(&bob, &under_languages)?, vec![ItemId::from("main.rs"), ItemId::from("notes.md")]);
//! # Ok::<(), folkengine::FolkError>(())
//! ```
//!
//! ## Purity, enforced
//!
//! `default = []`; the pure dependency tree is empty. `clippy.toml` bans the
//! filesystem, network, environment and clock; `deny.toml` keeps format and
//! runtime crates out of the graph; the `kernel-purity` CI job asserts both.

#![forbid(unsafe_code)]

mod action;
mod apply;
mod error;
mod event;
mod ids;
mod label;
mod query;
mod state;

pub use action::Action;
pub use apply::Transition;
pub use error::FolkError;
pub use event::Event;
pub use ids::{ActorId, ItemId, TagId};
pub use label::normalize_label;
pub use query::{FolksonomyView, Selection, TagCount, TagRef};
pub use state::{Binding, BindingKey, Defect, Folksonomy, Tag, Visibility};
