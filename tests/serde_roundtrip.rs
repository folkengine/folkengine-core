//! Opt-in serde derives exist on every public type. Gated on the feature;
//! no *format* crate enters even the dev graph — a shell picks the format.
#![cfg(feature = "serde")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use folkengine::{Action, Folksonomy, Visibility};
use serde::{Deserialize, Serialize};

/// A state plus a log, the shape a shell would persist.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Envelope {
    state: Folksonomy,
    actions: Vec<Action>,
}

#[test]
fn derives_exist_on_every_public_type() {
    let d = diamond();
    let s = tag(&d.s, "a", "x", d.rust, Visibility::Private);
    let env = Envelope {
        state: s,
        actions: vec![Action::RetireTag(d.rust)],
    };
    assert_serde(&env);
}

// Instantiating the bound is what checks every field type implements both
// traits; a format crate is not needed to prove that.
fn assert_serde<T: Serialize + for<'de> Deserialize<'de>>(_: &T) {}
