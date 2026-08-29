//! The pure transition.

use std::collections::BTreeSet;

use crate::{
    normalize_label, Action, ActorId, BindingKey, Event, FolkError, Folksonomy, ItemId, Tag, TagId,
    Visibility,
};

/// Result of a successful `apply`: the next state and, in order, every event
/// the action produced.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transition {
    /// The next state.
    pub state: Folksonomy,
    /// What happened, in order.
    pub events: Vec<Event>,
}

impl Folksonomy {
    /// The pure transition. Never mutates `self`; returns the next state and
    /// its events, or why the action was rejected. Rejection leaves nothing
    /// half-applied because nothing was applied.
    ///
    /// # Errors
    /// See [`FolkError`] — every arm is reachable from some action.
    pub fn apply(&self, actor: &ActorId, action: Action) -> Result<Transition, FolkError> {
        if action.is_structural() && !self.may_edit_structure(actor) {
            return Err(FolkError::NotAuthorized);
        }
        let mut next = self.clone();
        let mut events = Vec::new();
        match action {
            Action::DefineTag { label, parents } => {
                next.define_tag(actor, &label, parents, &mut events)?;
            }
            Action::RenameTag { tag, label } => next.rename_tag(actor, tag, &label, &mut events)?,
            Action::AddAlias { tag, alias } => next.add_alias(actor, tag, &alias, &mut events)?,
            Action::RemoveAlias { tag, alias } => {
                next.remove_alias(actor, tag, &alias, &mut events)?;
            }
            Action::AddParent { child, parent } => {
                next.add_parent(actor, child, parent, &mut events)?;
            }
            Action::RemoveParent { child, parent } => {
                next.remove_parent(actor, child, parent, &mut events)?;
            }
            Action::MergeTags { source, into } => {
                next.merge_tags(actor, source, into, &mut events)?;
            }
            Action::RetireTag(tag) => next.retire_tag(actor, tag, &mut events)?,
            Action::Tag {
                item,
                tag,
                visibility,
            } => next.tag_item(actor, item, tag, visibility, &mut events)?,
            Action::Untag { item, tag, tagger } => {
                next.untag_item(actor, item, tag, tagger, &mut events)?;
            }
        }
        Ok(Transition {
            state: next,
            events,
        })
    }

    fn valid_label(label: &str) -> Result<String, FolkError> {
        let norm = normalize_label(label);
        if norm.is_empty() {
            return Err(FolkError::InvalidLabel(label.to_owned()));
        }
        Ok(norm)
    }

    /// The normalized label if no *other* tag owns it.
    fn free_label(&self, label: &str, owner: Option<TagId>) -> Result<String, FolkError> {
        let norm = Self::valid_label(label)?;
        match self.resolve_normalized(&norm) {
            Some(t) if Some(t) != owner => Err(FolkError::DuplicateLabel(t)),
            _ => Ok(norm),
        }
    }

    fn define_tag(
        &mut self,
        actor: &ActorId,
        label: &str,
        parents: Vec<TagId>,
        events: &mut Vec<Event>,
    ) -> Result<(), FolkError> {
        let norm = self.free_label(label, None)?;
        let parents: BTreeSet<TagId> = parents.into_iter().collect();
        for p in &parents {
            self.require(*p)?;
        }
        let id = TagId(self.next_tag_id);
        self.next_tag_id += 1;
        self.tags.insert(
            id,
            Tag {
                id,
                label: norm.clone(),
                aliases: BTreeSet::new(),
                parents: parents.clone(),
            },
        );
        events.push(Event::TagDefined {
            tag: id,
            label: norm,
            parents: parents.into_iter().collect(),
            by: actor.clone(),
        });
        Ok(())
    }

    fn rename_tag(
        &mut self,
        actor: &ActorId,
        tag: TagId,
        label: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), FolkError> {
        self.require(tag)?;
        let norm = self.free_label(label, Some(tag))?;
        let t = self.tags.get_mut(&tag).ok_or(FolkError::UnknownTag(tag))?;
        if t.label == norm {
            return Ok(());
        }
        // Renaming to one of the tag's own aliases promotes it.
        t.aliases.remove(&norm);
        let old = std::mem::replace(&mut t.label, norm.clone());
        events.push(Event::TagRenamed {
            tag,
            old_label: old,
            new_label: norm,
            by: actor.clone(),
        });
        Ok(())
    }

    fn add_alias(
        &mut self,
        actor: &ActorId,
        tag: TagId,
        alias: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), FolkError> {
        self.require(tag)?;
        let norm = self.free_label(alias, Some(tag))?;
        let t = self.tags.get_mut(&tag).ok_or(FolkError::UnknownTag(tag))?;
        if t.label == norm || !t.aliases.insert(norm.clone()) {
            return Ok(());
        }
        events.push(Event::AliasAdded {
            tag,
            alias: norm,
            by: actor.clone(),
        });
        Ok(())
    }

    fn remove_alias(
        &mut self,
        actor: &ActorId,
        tag: TagId,
        alias: &str,
        events: &mut Vec<Event>,
    ) -> Result<(), FolkError> {
        let norm = normalize_label(alias);
        let t = self.tags.get_mut(&tag).ok_or(FolkError::UnknownTag(tag))?;
        if !t.aliases.remove(&norm) {
            return Err(FolkError::NotPresent);
        }
        events.push(Event::AliasRemoved {
            tag,
            alias: norm,
            by: actor.clone(),
        });
        Ok(())
    }

    fn add_parent(
        &mut self,
        actor: &ActorId,
        child: TagId,
        parent: TagId,
        events: &mut Vec<Event>,
    ) -> Result<(), FolkError> {
        self.require(child)?;
        self.require(parent)?;
        if self.require(child)?.parents.contains(&parent) {
            return Ok(());
        }
        if self.edge_would_cycle(child, parent) {
            return Err(FolkError::WouldCycle);
        }
        self.insert_edge(actor, child, parent, events);
        Ok(())
    }

    /// Unchecked edge insert; emits only if the edge was new.
    fn insert_edge(
        &mut self,
        actor: &ActorId,
        child: TagId,
        parent: TagId,
        events: &mut Vec<Event>,
    ) {
        if let Some(t) = self.tags.get_mut(&child) {
            if t.parents.insert(parent) {
                events.push(Event::ParentAdded {
                    child,
                    parent,
                    by: actor.clone(),
                });
            }
        }
    }

    fn remove_parent(
        &mut self,
        actor: &ActorId,
        child: TagId,
        parent: TagId,
        events: &mut Vec<Event>,
    ) -> Result<(), FolkError> {
        self.require(parent)?;
        let t = self
            .tags
            .get_mut(&child)
            .ok_or(FolkError::UnknownTag(child))?;
        if !t.parents.remove(&parent) {
            return Err(FolkError::NotPresent);
        }
        events.push(Event::ParentRemoved {
            child,
            parent,
            by: actor.clone(),
        });
        Ok(())
    }

    fn merge_tags(
        &mut self,
        actor: &ActorId,
        source: TagId,
        into: TagId,
        events: &mut Vec<Event>,
    ) -> Result<(), FolkError> {
        self.require(into)?;
        if source == into {
            return Err(FolkError::SelfMerge);
        }
        let src = self
            .tags
            .remove(&source)
            .ok_or(FolkError::UnknownTag(source))?;

        // 1. Edges: children re-parent, parents union.
        for child in self.children_of(source) {
            if let Some(t) = self.tags.get_mut(&child) {
                t.parents.remove(&source);
            }
            events.push(Event::ParentRemoved {
                child,
                parent: source,
                by: actor.clone(),
            });
            if child != into {
                self.insert_edge(actor, child, into, events);
            }
        }
        for p in &src.parents {
            if *p != into {
                self.insert_edge(actor, into, *p, events);
            }
        }
        if !self.cyclic_tags().is_empty() {
            return Err(FolkError::WouldCycle);
        }

        // 2. Labels: source's label and aliases become aliases of `into`.
        for alias in std::iter::once(&src.label).chain(src.aliases.iter()) {
            if let Some(t) = self.tags.get_mut(&into) {
                if t.label != *alias && t.aliases.insert(alias.clone()) {
                    events.push(Event::AliasAdded {
                        tag: into,
                        alias: alias.clone(),
                        by: actor.clone(),
                    });
                }
            }
        }

        // 3. Bindings move; duplicates are dropped and not counted.
        let moving: Vec<(BindingKey, Visibility)> = self
            .bindings
            .iter()
            .filter(|(k, _)| k.tag == source)
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        let mut moved: u32 = 0;
        for (key, vis) in moving {
            self.bindings.remove(&key);
            events.push(Event::Untagged {
                item: key.item.clone(),
                tag: source,
                tagger: key.tagger.clone(),
                visibility: vis,
            });
            let target = BindingKey {
                item: key.item,
                tag: into,
                tagger: key.tagger,
            };
            if self.bindings.contains_key(&target) {
                continue;
            }
            self.bindings.insert(target.clone(), vis);
            moved += 1;
            events.push(Event::Tagged {
                item: target.item,
                tag: into,
                tagger: target.tagger,
                visibility: vis,
            });
        }

        events.push(Event::TagsMerged {
            source,
            into,
            moved_bindings: moved,
            by: actor.clone(),
        });
        Ok(())
    }

    fn retire_tag(
        &mut self,
        actor: &ActorId,
        tag: TagId,
        events: &mut Vec<Event>,
    ) -> Result<(), FolkError> {
        self.require(tag)?;
        if self.has_bindings(tag) || self.has_children(tag) {
            return Err(FolkError::TagInUse(tag));
        }
        let t = self.tags.remove(&tag).ok_or(FolkError::UnknownTag(tag))?;
        events.push(Event::TagRetired {
            tag,
            label: t.label,
            by: actor.clone(),
        });
        Ok(())
    }

    fn tag_item(
        &mut self,
        actor: &ActorId,
        item: ItemId,
        tag: TagId,
        visibility: Visibility,
        events: &mut Vec<Event>,
    ) -> Result<(), FolkError> {
        self.require(tag)?;
        let key = BindingKey {
            item,
            tag,
            tagger: actor.clone(),
        };
        match self.bindings.insert(key.clone(), visibility) {
            Some(prev) if prev == visibility => {}
            Some(_) => events.push(Event::VisibilityChanged {
                item: key.item,
                tag,
                tagger: key.tagger,
                visibility,
            }),
            None => events.push(Event::Tagged {
                item: key.item,
                tag,
                tagger: key.tagger,
                visibility,
            }),
        }
        Ok(())
    }

    fn untag_item(
        &mut self,
        actor: &ActorId,
        item: ItemId,
        tag: TagId,
        tagger: Option<ActorId>,
        events: &mut Vec<Event>,
    ) -> Result<(), FolkError> {
        self.require(tag)?;
        let tagger = tagger.unwrap_or_else(|| actor.clone());
        if tagger != *actor && !self.is_curator(actor) {
            return Err(FolkError::NotAuthorized);
        }
        let key = BindingKey { item, tag, tagger };
        let vis = self.bindings.remove(&key).ok_or(FolkError::NotBound)?;
        events.push(Event::Untagged {
            item: key.item,
            tag,
            tagger: key.tagger,
            visibility: vis,
        });
        Ok(())
    }
}
