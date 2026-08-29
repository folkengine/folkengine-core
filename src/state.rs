//! Kernel state: the vocabulary, the bindings, and governance-as-data.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{normalize_label, ActorId, FolkError, ItemId, TagId};

/// Who may see a binding. A private binding is visible only to its tagger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Visibility {
    /// Visible to every actor.
    Public,
    /// Visible only to the tagger.
    Private,
}

/// A concept in the vocabulary. `label` is the preferred label, `aliases` are
/// alternative labels that resolve to it; both are stored normalized.
/// `parents` makes the vocabulary a DAG: several parents are allowed, cycles
/// are not.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tag {
    /// Stable identity.
    pub id: TagId,
    /// Preferred label, normalized.
    pub label: String,
    /// Alternative labels, normalized.
    pub aliases: BTreeSet<String>,
    /// Broader tags.
    pub parents: BTreeSet<TagId>,
}

/// The folksonomy atom: `(tagger, item, tag)` plus who may see it. The same
/// item may carry the same tag from many taggers; each is its own binding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Binding {
    /// Bound item.
    pub item: ItemId,
    /// Bound tag.
    pub tag: TagId,
    /// Whose binding.
    pub tagger: ActorId,
    /// Who may see it.
    pub visibility: Visibility,
}

/// Identity of a binding — everything but its visibility.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BindingKey {
    /// Bound item.
    pub item: ItemId,
    /// Bound tag.
    pub tag: TagId,
    /// Whose binding.
    pub tagger: ActorId,
}

/// Full kernel state. Value semantics: `apply` never mutates its receiver.
///
/// `curators` is governance-as-data, owned and set by the shell that owns the
/// state. When non-empty, structural vocabulary edits require a curator;
/// defining a tag and tagging are always open. There is no action that
/// changes the curator list — that bootstrap rule is the shell's decision.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Folksonomy {
    pub(crate) next_tag_id: u64,
    pub(crate) tags: BTreeMap<TagId, Tag>,
    pub(crate) bindings: BTreeMap<BindingKey, Visibility>,
    pub(crate) curators: BTreeSet<ActorId>,
}

/// A way a loaded state can violate the kernel's invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Defect {
    /// A tag lists a parent that does not exist.
    DanglingParent {
        /// The tag with the bad edge.
        child: TagId,
        /// The missing parent.
        parent: TagId,
    },
    /// These tags participate in at least one cycle.
    Cycle(Vec<TagId>),
    /// A label or alias is owned by more than one tag.
    DuplicateLabel(String),
    /// A binding references a tag that does not exist.
    DanglingBinding(Binding),
    /// `next_tag_id` is not greater than every existing id.
    IdCounterBehind,
}

impl Folksonomy {
    /// An empty folksonomy with the given curators. An empty curator list
    /// means a fully open vocabulary.
    #[must_use]
    pub fn empty<I, A>(curators: I) -> Self
    where
        I: IntoIterator<Item = A>,
        A: Into<ActorId>,
    {
        Self {
            next_tag_id: 1,
            tags: BTreeMap::new(),
            bindings: BTreeMap::new(),
            curators: curators.into_iter().map(Into::into).collect(),
        }
    }

    /// Reconstruct a state from its parts, e.g. after deserializing a
    /// projection-free dump. Run [`Folksonomy::validate`] on the result before
    /// trusting it.
    #[must_use]
    pub fn from_parts(
        next_tag_id: u64,
        tags: impl IntoIterator<Item = Tag>,
        bindings: impl IntoIterator<Item = Binding>,
        curators: impl IntoIterator<Item = ActorId>,
    ) -> Self {
        Self {
            next_tag_id,
            tags: tags.into_iter().map(|t| (t.id, t)).collect(),
            bindings: bindings
                .into_iter()
                .map(|b| {
                    (
                        BindingKey {
                            item: b.item,
                            tag: b.tag,
                            tagger: b.tagger,
                        },
                        b.visibility,
                    )
                })
                .collect(),
            curators: curators.into_iter().collect(),
        }
    }

    /// The id the next `DefineTag` will allocate.
    #[must_use]
    pub fn next_tag_id(&self) -> TagId {
        TagId(self.next_tag_id)
    }

    /// The vocabulary, by id.
    pub fn tags(&self) -> impl Iterator<Item = &Tag> {
        self.tags.values()
    }

    /// One tag.
    #[must_use]
    pub fn tag(&self, id: TagId) -> Option<&Tag> {
        self.tags.get(&id)
    }

    /// Every binding, in `(item, tag, tagger)` order, regardless of actor.
    /// Shells that hold the state may read this; anything shown to an actor
    /// should go through [`Folksonomy::view_for`](crate::Folksonomy::view_for).
    pub fn bindings(&self) -> impl Iterator<Item = Binding> + '_ {
        self.bindings.iter().map(|(k, v)| Binding {
            item: k.item.clone(),
            tag: k.tag,
            tagger: k.tagger.clone(),
            visibility: *v,
        })
    }

    /// The curators. Empty means open.
    #[must_use]
    pub fn curators(&self) -> &BTreeSet<ActorId> {
        &self.curators
    }

    /// Whether this actor is a curator.
    #[must_use]
    pub fn is_curator(&self, actor: &ActorId) -> bool {
        self.curators.contains(actor)
    }

    /// Whether this actor may perform structural edits: always when the
    /// vocabulary is open, otherwise only curators.
    #[must_use]
    pub fn may_edit_structure(&self, actor: &ActorId) -> bool {
        self.curators.is_empty() || self.is_curator(actor)
    }

    /// Label or alias → tag, after normalization.
    #[must_use]
    pub fn resolve(&self, label: &str) -> Option<TagId> {
        let norm = normalize_label(label);
        self.resolve_normalized(&norm)
    }

    pub(crate) fn resolve_normalized(&self, norm: &str) -> Option<TagId> {
        self.tags
            .values()
            .find(|t| t.label == norm || t.aliases.contains(norm))
            .map(|t| t.id)
    }

    pub(crate) fn require(&self, id: TagId) -> Result<&Tag, FolkError> {
        self.tags.get(&id).ok_or(FolkError::UnknownTag(id))
    }

    /// Transitive closure upward, excluding the tag itself, sorted by id.
    ///
    /// # Errors
    /// [`FolkError::UnknownTag`] if the tag does not exist.
    pub fn ancestors(&self, tag: TagId) -> Result<Vec<TagId>, FolkError> {
        self.require(tag)?;
        Ok(Self::closure(tag, |t| self.parents_of(t))
            .into_iter()
            .collect())
    }

    /// Transitive closure downward, excluding the tag itself, sorted by id.
    ///
    /// # Errors
    /// [`FolkError::UnknownTag`] if the tag does not exist.
    pub fn descendants(&self, tag: TagId) -> Result<Vec<TagId>, FolkError> {
        self.require(tag)?;
        let children = self.children_index();
        Ok(Self::closure(tag, |t| {
            children.get(&t).into_iter().flatten().copied().collect()
        })
        .into_iter()
        .collect())
    }

    /// Direct children of a tag (tags listing it as a parent), sorted.
    pub(crate) fn children_of(&self, tag: TagId) -> Vec<TagId> {
        self.tags
            .values()
            .filter(|t| t.parents.contains(&tag))
            .map(|t| t.id)
            .collect()
    }

    pub(crate) fn has_children(&self, tag: TagId) -> bool {
        self.tags.values().any(|t| t.parents.contains(&tag))
    }

    pub(crate) fn has_bindings(&self, tag: TagId) -> bool {
        self.bindings.keys().any(|k| k.tag == tag)
    }

    fn parents_of(&self, tag: TagId) -> Vec<TagId> {
        self.tags
            .get(&tag)
            .map(|t| t.parents.iter().copied().collect())
            .unwrap_or_default()
    }

    pub(crate) fn children_index(&self) -> BTreeMap<TagId, Vec<TagId>> {
        let mut idx: BTreeMap<TagId, Vec<TagId>> = BTreeMap::new();
        for t in self.tags.values() {
            for p in &t.parents {
                idx.entry(*p).or_default().push(t.id);
            }
        }
        idx
    }

    /// Set of tags reachable from `start` via `next`, excluding `start`
    /// unless it is reachable from itself (a cycle).
    fn closure(start: TagId, next: impl Fn(TagId) -> Vec<TagId>) -> BTreeSet<TagId> {
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::from([start]);
        while let Some(t) = queue.pop_front() {
            for n in next(t) {
                if seen.insert(n) {
                    queue.push_back(n);
                }
            }
        }
        seen
    }

    /// Whether adding the edge `child -> parent` would close a cycle:
    /// true when `child` is already an ancestor of `parent` (or is `parent`).
    pub(crate) fn edge_would_cycle(&self, child: TagId, parent: TagId) -> bool {
        child == parent || Self::closure(parent, |t| self.parents_of(t)).contains(&child)
    }

    /// Tags on at least one cycle, sorted (Kahn's algorithm residue).
    pub(crate) fn cyclic_tags(&self) -> Vec<TagId> {
        let mut indeg: BTreeMap<TagId, usize> = self.tags.keys().map(|k| (*k, 0)).collect();
        // Edge direction for the sort: parent -> child.
        for t in self.tags.values() {
            for p in &t.parents {
                if self.tags.contains_key(p) {
                    *indeg.entry(t.id).or_default() += 1;
                }
            }
        }
        let children = self.children_index();
        let mut queue: VecDeque<TagId> = indeg
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(k, _)| *k)
            .collect();
        while let Some(t) = queue.pop_front() {
            for c in children.get(&t).into_iter().flatten() {
                if let Some(d) = indeg.get_mut(c) {
                    *d -= 1;
                    if *d == 0 {
                        queue.push_back(*c);
                    }
                }
            }
        }
        indeg
            .into_iter()
            .filter(|(_, d)| *d > 0)
            .map(|(k, _)| k)
            .collect()
    }

    /// Check a state the shell loaded from somewhere against the kernel's
    /// invariants. An empty result means `apply` and the queries are safe.
    #[must_use]
    pub fn validate(&self) -> Vec<Defect> {
        let mut defects = Vec::new();
        for t in self.tags.values() {
            for p in &t.parents {
                if !self.tags.contains_key(p) {
                    defects.push(Defect::DanglingParent {
                        child: t.id,
                        parent: *p,
                    });
                }
            }
        }
        let cyclic = self.cyclic_tags();
        if !cyclic.is_empty() {
            defects.push(Defect::Cycle(cyclic));
        }
        let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
        for t in self.tags.values() {
            *seen.entry(t.label.as_str()).or_default() += 1;
            for a in &t.aliases {
                *seen.entry(a.as_str()).or_default() += 1;
            }
        }
        defects.extend(
            seen.into_iter()
                .filter(|(_, n)| *n > 1)
                .map(|(l, _)| Defect::DuplicateLabel(l.to_owned())),
        );
        for b in self.bindings() {
            if !self.tags.contains_key(&b.tag) {
                defects.push(Defect::DanglingBinding(b));
            }
        }
        if self
            .tags
            .keys()
            .next_back()
            .is_some_and(|max| max.0 >= self.next_tag_id)
        {
            defects.push(Defect::IdCounterBehind);
        }
        defects
    }
}
