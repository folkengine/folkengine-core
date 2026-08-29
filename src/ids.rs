//! The three identities of the domain, each with a different owner.

use core::fmt;

/// Stable identity of a tag, allocated by the kernel from
/// [`Folksonomy::next_tag_id`](crate::Folksonomy::next_tag_id). Labels may
/// change; ids never do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct TagId(pub u64);

impl fmt::Display for TagId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Opaque, caller-supplied identity of a tagged thing. The kernel compares it
/// bytewise and does nothing else with it: a path, a URL, a content digest or
/// a database key are all the shell's business.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ItemId(pub String);

/// Opaque, caller-supplied identity of the party performing an action — the
/// "folk" in folksonomy. The kernel is told who the actor is; it never finds out.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ActorId(pub String);

macro_rules! string_id {
    ($t:ident) => {
        impl From<&str> for $t {
            fn from(s: &str) -> Self {
                Self(s.to_owned())
            }
        }
        impl From<String> for $t {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
        impl AsRef<str> for $t {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(ItemId);
string_id!(ActorId);
