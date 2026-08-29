//! Outputs of [`Folksonomy::apply`](crate::Folksonomy::apply) — the product.
//!
//! Events are structured values, never sentences. Shells persist them, mirror
//! them into indexes, xattrs or sidecars, or translate them into another
//! kernel's actions. The order within one `apply` is part of the contract.

use crate::{ActorId, ItemId, TagId, Visibility};

/// Everything `apply` can report. Mirrors `event` in the WIT.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Event {
    /// A new tag exists.
    TagDefined {
        /// The allocated id.
        tag: TagId,
        /// Normalized label.
        label: String,
        /// Parents at definition, sorted.
        parents: Vec<TagId>,
        /// Who defined it.
        by: ActorId,
    },
    /// The preferred label changed.
    TagRenamed {
        /// Renamed tag.
        tag: TagId,
        /// Previous normalized label.
        old_label: String,
        /// New normalized label.
        new_label: String,
        /// Who renamed it.
        by: ActorId,
    },
    /// An alias now resolves to the tag.
    AliasAdded {
        /// Tag owning the alias.
        tag: TagId,
        /// Normalized alias.
        alias: String,
        /// Who added it.
        by: ActorId,
    },
    /// An alias no longer resolves.
    AliasRemoved {
        /// Tag that owned the alias.
        tag: TagId,
        /// Normalized alias.
        alias: String,
        /// Who removed it.
        by: ActorId,
    },
    /// A parent edge now exists.
    ParentAdded {
        /// Narrower tag.
        child: TagId,
        /// Broader tag.
        parent: TagId,
        /// Who added it.
        by: ActorId,
    },
    /// A parent edge no longer exists.
    ParentRemoved {
        /// Narrower tag.
        child: TagId,
        /// Broader tag.
        parent: TagId,
        /// Who removed it.
        by: ActorId,
    },
    /// Summary emitted last by a merge, after its primitive events.
    TagsMerged {
        /// Tag that disappeared.
        source: TagId,
        /// Tag that absorbed it.
        into: TagId,
        /// Bindings that moved (duplicates dropped are not counted).
        moved_bindings: u32,
        /// Who merged.
        by: ActorId,
    },
    /// A tag was removed.
    TagRetired {
        /// Removed tag.
        tag: TagId,
        /// Its label at removal.
        label: String,
        /// Who retired it.
        by: ActorId,
    },
    /// A binding now exists.
    Tagged {
        /// Bound item.
        item: ItemId,
        /// Bound tag.
        tag: TagId,
        /// Whose binding.
        tagger: ActorId,
        /// Its visibility.
        visibility: Visibility,
    },
    /// A binding no longer exists.
    Untagged {
        /// Unbound item.
        item: ItemId,
        /// Unbound tag.
        tag: TagId,
        /// Whose binding it was.
        tagger: ActorId,
        /// Its visibility at removal.
        visibility: Visibility,
    },
    /// A binding's visibility changed.
    VisibilityChanged {
        /// Bound item.
        item: ItemId,
        /// Bound tag.
        tag: TagId,
        /// Whose binding.
        tagger: ActorId,
        /// The new visibility.
        visibility: Visibility,
    },
}
