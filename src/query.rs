//! Pure reads. Every read takes an actor and is therefore already a
//! projection: it sees public bindings plus the actor's own private ones.

use std::collections::{BTreeMap, BTreeSet};

use crate::{ActorId, Binding, FolkError, Folksonomy, ItemId, Tag, TagId, Visibility};

/// A reference to a tag in a selection. `transitive` means the tag or any of
/// its descendants; otherwise exactly this tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TagRef {
    /// The tag.
    pub tag: TagId,
    /// Include descendants.
    pub transitive: bool,
}

impl TagRef {
    /// Exactly this tag.
    #[must_use]
    pub fn exact(tag: TagId) -> Self {
        Self {
            tag,
            transitive: false,
        }
    }

    /// This tag or any descendant.
    #[must_use]
    pub fn under(tag: TagId) -> Self {
        Self {
            tag,
            transitive: true,
        }
    }
}

/// Disjunctive normal form, flattened because WIT types cannot recurse. An
/// item matches if it satisfies *every* ref in *some* clause of `any_of` and
/// *no* ref in `none_of`. An empty `any_of` matches every item with at least
/// one visible binding. `taggers`, when present, restricts the bindings
/// considered to those taggers before evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Selection {
    /// Clauses; each is a conjunction.
    pub any_of: Vec<Vec<TagRef>>,
    /// Exclusions.
    pub none_of: Vec<TagRef>,
    /// Restrict to bindings by these taggers.
    pub taggers: Option<Vec<ActorId>>,
}

impl Selection {
    /// Everything with at least one visible binding.
    #[must_use]
    pub fn everything() -> Self {
        Self::default()
    }

    /// A single-clause selection: all of these refs.
    #[must_use]
    pub fn all_of(refs: impl IntoIterator<Item = TagRef>) -> Self {
        Self {
            any_of: vec![refs.into_iter().collect()],
            ..Self::default()
        }
    }

    /// Add an exclusion.
    #[must_use]
    pub fn but_not(mut self, r: TagRef) -> Self {
        self.none_of.push(r);
        self
    }

    /// Restrict to these taggers.
    #[must_use]
    pub fn by(mut self, taggers: impl IntoIterator<Item = ActorId>) -> Self {
        self.taggers = Some(taggers.into_iter().collect());
        self
    }
}

/// How many distinct matching items carry a tag directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TagCount {
    /// The tag.
    pub tag: TagId,
    /// Distinct items.
    pub count: u32,
}

/// What one actor is entitled to see: the whole vocabulary, public bindings,
/// and its own private bindings. Deliberately a different type from
/// [`Folksonomy`] so a projection cannot be mistaken for state.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FolksonomyView {
    /// The vocabulary, sorted by id.
    pub tags: Vec<Tag>,
    /// Visible bindings, sorted by `(item, tag, tagger)`.
    pub bindings: Vec<Binding>,
}

impl Folksonomy {
    /// The visibility rule, in one place: public, or the actor's own.
    fn visible_to(actor: &ActorId, b: &Binding) -> bool {
        b.visibility == Visibility::Public || b.tagger == *actor
    }

    /// Hidden-information projection. The single place the visibility rule
    /// lives; every other read applies it.
    #[must_use]
    pub fn view_for(&self, actor: &ActorId) -> FolksonomyView {
        FolksonomyView {
            tags: self.tags.values().cloned().collect(),
            bindings: self
                .bindings()
                .filter(|b| Self::visible_to(actor, b))
                .collect(),
        }
    }

    /// Bindings on one item visible to `actor`, sorted by `(tag, tagger)`.
    #[must_use]
    pub fn tags_of(&self, actor: &ActorId, item: &ItemId) -> Vec<Binding> {
        self.bindings()
            .filter(|b| b.item == *item && Self::visible_to(actor, b))
            .collect()
    }

    /// Item → set of tags directly bound to it, over the bindings `actor`
    /// may see, optionally restricted to `taggers`.
    fn visible_index(
        &self,
        actor: &ActorId,
        taggers: Option<&[ActorId]>,
    ) -> BTreeMap<ItemId, BTreeSet<TagId>> {
        let mut idx: BTreeMap<ItemId, BTreeSet<TagId>> = BTreeMap::new();
        for b in self.bindings().filter(|b| Self::visible_to(actor, b)) {
            if taggers.is_some_and(|ts| !ts.contains(&b.tagger)) {
                continue;
            }
            idx.entry(b.item).or_default().insert(b.tag);
        }
        idx
    }

    /// The set of tags a ref matches: itself, plus descendants if transitive.
    fn expand(&self, r: TagRef) -> Result<BTreeSet<TagId>, FolkError> {
        let mut set = BTreeSet::from([r.tag]);
        if r.transitive {
            set.extend(self.descendants(r.tag)?);
        } else {
            self.require(r.tag)?;
        }
        Ok(set)
    }

    fn matches(
        &self,
        sel: &Selection,
        actor: &ActorId,
    ) -> Result<BTreeMap<ItemId, BTreeSet<TagId>>, FolkError> {
        let clauses: Vec<Vec<BTreeSet<TagId>>> = sel
            .any_of
            .iter()
            .map(|c| c.iter().map(|r| self.expand(*r)).collect())
            .collect::<Result<_, _>>()?;
        let excluded: Vec<BTreeSet<TagId>> = sel
            .none_of
            .iter()
            .map(|r| self.expand(*r))
            .collect::<Result<_, _>>()?;
        let idx = self.visible_index(actor, sel.taggers.as_deref());
        let hit = |tags: &BTreeSet<TagId>, want: &BTreeSet<TagId>| !tags.is_disjoint(want);
        Ok(idx
            .into_iter()
            .filter(|(_, tags)| {
                let included = clauses.is_empty()
                    || clauses
                        .iter()
                        .any(|clause| clause.iter().all(|want| hit(tags, want)));
                included && !excluded.iter().any(|want| hit(tags, want))
            })
            .collect())
    }

    /// Items matching `sel` as seen by `actor`, deduplicated and sorted
    /// bytewise by item id.
    ///
    /// # Errors
    /// [`FolkError::UnknownTag`] if the selection names a tag that does not
    /// exist.
    pub fn query(&self, actor: &ActorId, sel: &Selection) -> Result<Vec<ItemId>, FolkError> {
        Ok(self.matches(sel, actor)?.into_keys().collect())
    }

    /// For the items `sel` matches, how many distinct items carry each tag
    /// directly (not transitively). Sorted by count descending, then tag id
    /// ascending. The "narrow further" affordance.
    ///
    /// # Errors
    /// [`FolkError::UnknownTag`] if the selection names a tag that does not
    /// exist.
    pub fn facets(&self, actor: &ActorId, sel: &Selection) -> Result<Vec<TagCount>, FolkError> {
        let mut counts: BTreeMap<TagId, u32> = BTreeMap::new();
        for tags in self.matches(sel, actor)?.into_values() {
            for t in tags {
                *counts.entry(t).or_default() += 1;
            }
        }
        let mut out: Vec<TagCount> = counts
            .into_iter()
            .map(|(tag, count)| TagCount { tag, count })
            .collect();
        out.sort_by(|a, b| b.count.cmp(&a.count).then(a.tag.cmp(&b.tag)));
        Ok(out)
    }
}
