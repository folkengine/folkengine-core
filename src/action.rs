//! Inputs to [`Folksonomy::apply`](crate::Folksonomy::apply).

use crate::{ActorId, ItemId, TagId, Visibility};

/// Every transition the kernel accepts. Mirrors `action` in the WIT.
///
/// *Open* actions may be performed by any actor. *Curated* actions require a
/// curator when [`Folksonomy::curators`](crate::Folksonomy::curators) is
/// non-empty and are open otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Action {
    /// Open. Create a tag with a label and zero or more existing parents.
    DefineTag {
        /// Label as typed; the kernel normalizes it.
        label: String,
        /// Parents, which must already exist. Duplicates are collapsed.
        parents: Vec<TagId>,
    },
    /// Curated. Change the preferred label. The old label does **not**
    /// become an alias automatically; use [`Action::AddAlias`].
    RenameTag {
        /// Tag to rename.
        tag: TagId,
        /// New label as typed.
        label: String,
    },
    /// Curated. Add an alternative label that resolves to this tag.
    AddAlias {
        /// Tag receiving the alias.
        tag: TagId,
        /// Alias as typed.
        alias: String,
    },
    /// Curated. Remove an alias.
    RemoveAlias {
        /// Tag owning the alias.
        tag: TagId,
        /// Alias as typed.
        alias: String,
    },
    /// Curated. Add a parent edge. Rejected if it would create a cycle.
    AddParent {
        /// The narrower tag.
        child: TagId,
        /// The broader tag.
        parent: TagId,
    },
    /// Curated. Remove a parent edge.
    RemoveParent {
        /// The narrower tag.
        child: TagId,
        /// The broader tag.
        parent: TagId,
    },
    /// Curated. Fold `source` into `into`: bindings move, `source`'s label and
    /// aliases become aliases of `into`, parents union, children re-parent,
    /// `source` is removed. Rejected if the result would be cyclic.
    MergeTags {
        /// Tag that disappears.
        source: TagId,
        /// Tag that absorbs it.
        into: TagId,
    },
    /// Curated. Remove a tag that has no bindings and no children.
    RetireTag(TagId),
    /// Open. Bind `tag` to `item` on behalf of the acting actor. Idempotent.
    Tag {
        /// Item to tag.
        item: ItemId,
        /// Tag to apply.
        tag: TagId,
        /// Who may see this binding.
        visibility: Visibility,
    },
    /// Open for one's own bindings; a curator may name another tagger.
    Untag {
        /// Item to untag.
        item: ItemId,
        /// Tag to remove.
        tag: TagId,
        /// Whose binding; `None` means the acting actor's own.
        tagger: Option<ActorId>,
    },
}

impl Action {
    /// Whether this action is structural (curated) rather than open.
    #[must_use]
    pub fn is_structural(&self) -> bool {
        !matches!(
            self,
            Self::DefineTag { .. } | Self::Tag { .. } | Self::Untag { .. }
        )
    }
}
