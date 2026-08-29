//! Why an action was rejected. `apply` returning a typed error is the
//! legality oracle — there is no `legal-actions` because the action space
//! (free-text labels) is unbounded.

use core::fmt;

use crate::TagId;

/// Every way `apply` or a query can refuse. Mirrors `folk-error` in the WIT.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FolkError {
    /// The referenced tag does not exist.
    UnknownTag(TagId),
    /// The label is empty after normalization; payload is the label as given.
    InvalidLabel(String),
    /// Another tag already owns this label or alias; payload is that tag.
    DuplicateLabel(TagId),
    /// The edit would make the vocabulary cyclic.
    WouldCycle,
    /// A structural edit, or an untag of another actor's binding, by a
    /// non-curator.
    NotAuthorized,
    /// Retire refused: the tag still has bindings or children.
    TagInUse(TagId),
    /// Merging a tag into itself.
    SelfMerge,
    /// Untag of a binding that does not exist.
    NotBound,
    /// Remove-parent / remove-alias of an edge or alias that does not exist.
    NotPresent,
}

impl fmt::Display for FolkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTag(t) => write!(f, "unknown tag {t}"),
            Self::InvalidLabel(l) => write!(f, "label {l:?} is empty after normalization"),
            Self::DuplicateLabel(t) => write!(f, "label already owned by tag {t}"),
            Self::WouldCycle => f.write_str("edit would make the vocabulary cyclic"),
            Self::NotAuthorized => f.write_str("actor is not authorized for this action"),
            Self::TagInUse(t) => write!(f, "tag {t} still has bindings or children"),
            Self::SelfMerge => f.write_str("cannot merge a tag into itself"),
            Self::NotBound => f.write_str("no such binding"),
            Self::NotPresent => f.write_str("no such parent edge or alias"),
        }
    }
}

impl std::error::Error for FolkError {}
