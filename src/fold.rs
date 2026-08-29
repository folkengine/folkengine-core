//! Replay — the other half of the transition contract.
//!
//! [`Folksonomy::apply`] returns a [`Transition`](crate::Transition) with two
//! halves: the next state and the events that describe it. Nothing forces the
//! two to agree unless something replays one into the other. That something is
//! [`Folksonomy::fold`], and the equation it makes testable —
//!
//! ```text
//! pre.fold(&pre.apply(actor, action)?.events) == pre.apply(actor, action)?.state
//! ```
//!
//! — is what lets a shell treat the event stream as authoritative and the state
//! as a disposable cache. If the equation ever fails, a rebuilt index is not the
//! state it claims to rebuild.
//!
//! `fold` is a *replay*, not a transition: it asks no questions about
//! authorization or legality, because those were answered by the `apply` that
//! produced the events. Folding events that no `apply` emitted, or folding them
//! onto the wrong prior state, can produce a state that
//! [`Folksonomy::validate`] rejects. That is the caller's problem, and it is why
//! `fold` is infallible rather than helpfully wrong.

use std::collections::BTreeSet;

use crate::{BindingKey, Event, Folksonomy, Tag};

impl Folksonomy {
    /// Replay events onto this state, returning the result.
    ///
    /// Never mutates its receiver. `curators` is carried through untouched:
    /// no event changes it, so the caller must fold onto a state that already
    /// has the right curator set (see the genesis note on
    /// [`Folksonomy::empty`]).
    ///
    /// ```
    /// use folkengine::{Action, ActorId, Folksonomy, ItemId, Visibility};
    ///
    /// let alice = ActorId::from("alice");
    /// let pre = Folksonomy::empty(Vec::<ActorId>::new());
    /// let t = pre.apply(&alice, Action::DefineTag { label: "Rust".into(), parents: vec![] })?;
    ///
    /// // The state half and the event half agree.
    /// assert_eq!(pre.fold(&t.events), t.state);
    /// # Ok::<(), folkengine::FolkError>(())
    /// ```
    #[must_use]
    pub fn fold(&self, events: &[Event]) -> Self {
        let mut next = self.clone();
        for e in events {
            next.replay(e);
        }
        next
    }

    /// One event. Every arm here is the inverse of the code in `apply` that
    /// emits it; the property test in `tests/fold.rs` is what keeps them
    /// honest.
    fn replay(&mut self, event: &Event) {
        match event {
            Event::TagDefined {
                tag,
                label,
                parents,
                ..
            } => {
                self.tags.insert(
                    *tag,
                    Tag {
                        id: *tag,
                        label: label.clone(),
                        aliases: BTreeSet::new(),
                        parents: parents.iter().copied().collect(),
                    },
                );
                // Allocation is a counter, so replay must advance it past any
                // id it has seen or the next `DefineTag` collides.
                self.next_tag_id = self.next_tag_id.max(tag.0 + 1);
            }
            Event::TagRenamed { tag, new_label, .. } => {
                if let Some(t) = self.tags.get_mut(tag) {
                    // `rename_tag` promotes: renaming to one of the tag's own
                    // aliases drops it from the alias set.
                    t.aliases.remove(new_label);
                    t.label.clone_from(new_label);
                }
            }
            Event::AliasAdded { tag, alias, .. } => {
                if let Some(t) = self.tags.get_mut(tag) {
                    t.aliases.insert(alias.clone());
                }
            }
            Event::AliasRemoved { tag, alias, .. } => {
                if let Some(t) = self.tags.get_mut(tag) {
                    t.aliases.remove(alias);
                }
            }
            Event::ParentAdded { child, parent, .. } => {
                if let Some(t) = self.tags.get_mut(child) {
                    t.parents.insert(*parent);
                }
            }
            Event::ParentRemoved { child, parent, .. } => {
                if let Some(t) = self.tags.get_mut(child) {
                    t.parents.remove(parent);
                }
            }
            // A merge emits a primitive event for every rewired edge, absorbed
            // label and moved binding; the summary carries exactly one fact the
            // primitives do not — that `source` is gone.
            Event::TagsMerged { source, .. } => {
                self.tags.remove(source);
            }
            Event::TagRetired { tag, .. } => {
                self.tags.remove(tag);
            }
            // Binding identity is (item, tag, tagger); visibility is the value,
            // so a first bind and a revision are the same insert.
            Event::Tagged {
                item,
                tag,
                tagger,
                visibility,
            }
            | Event::VisibilityChanged {
                item,
                tag,
                tagger,
                visibility,
            } => {
                self.bindings.insert(
                    BindingKey {
                        item: item.clone(),
                        tag: *tag,
                        tagger: tagger.clone(),
                    },
                    *visibility,
                );
            }
            Event::Untagged {
                item, tag, tagger, ..
            } => {
                self.bindings.remove(&BindingKey {
                    item: item.clone(),
                    tag: *tag,
                    tagger: tagger.clone(),
                });
            }
        }
    }
}
