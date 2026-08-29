# folkengine

A folksonomy **domain kernel**: the pure logic of a universal tagging engine —
a poly-hierarchical tag vocabulary, `(tagger, item, tag)` bindings, and
per-actor selections over them. No I/O, no clock, no files. Items are opaque
ids; what they point at is the shell's business.

The design charter is `FOLKENGINE_CHARTER.md`. The language-neutral contract
is `wit/folkengine.wit`; this crate implements it one-to-one.

```rust
use folkengine::{Action, ActorId, Folksonomy, ItemId, Selection, TagRef, Visibility};

let me = ActorId::from("me");
let s = Folksonomy::empty(Vec::<ActorId>::new());          // open vocabulary
let t = s.apply(&me, Action::DefineTag { label: "Languages".into(), parents: vec![] })?;
let languages = t.state.resolve("languages").unwrap();
let t = t.state.apply(&me, Action::DefineTag { label: "Rust".into(), parents: vec![languages] })?;
let rust = t.state.resolve("rust").unwrap();
let t = t.state.apply(&me, Action::Tag { item: ItemId::from("main.rs"), tag: rust, visibility: Visibility::Public })?;

let hits = t.state.query(&me, &Selection::all_of([TagRef::under(languages)]))?;
assert_eq!(hits, vec![ItemId::from("main.rs")]);          // transitive: rust ⊂ languages
# Ok::<(), folkengine::FolkError>(())
```

## Surface

| | |
|---|---|
| `Folksonomy::empty(curators)` | blank state; empty curators = fully open |
| `normalize_label` | the one label rule: trim, single-space, Unicode lowercase |
| `validate` | defects in a shell-loaded state |
| `apply(actor, action) -> Result<Transition, FolkError>` | the pure transition: next state + ordered events |
| `fold(events) -> Folksonomy` | replay: the events of a transition, folded back into its state |
| `resolve`, `ancestors`, `descendants` | vocabulary reads |
| `query`, `facets`, `tags_of` | selection reads, always as seen by an actor |
| `view_for(actor)` | the hidden-information projection; the one place the visibility rule lives |

## Purity, enforced

`default = []`. The pure dependency tree is one line:

```
$ cargo tree --no-default-features
folkengine v0.1.0
```

| Gate | Command |
|---|---|
| Lints (pedantic, warnings denied) | `cargo clippy --all-targets -- -D warnings` |
| Banned std entry points | `clippy.toml` — filesystem, network, env, process, clock, `HashMap`/`HashSet` |
| Banned crates in the graph | `cargo deny check bans` — formats, runtimes, transports, stores, entropy, clocks |
| Pure build + tests | `cargo test --no-default-features` |
| The transition contract | `cargo test --no-default-features --test fold` |
| Grep-level purity | `python3 scripts/check_purity.py .` |
| Contract still resolves | `componentize-py -d wit/folkengine.wit -w folkengine bindings out/` |

`.github/workflows/kernel-purity.yml` runs all of them.

## Features

- `serde` — derives on every public type. `serde` is a trait crate; concrete
  formats are banned from the graph and belong to a shell.
- `full` — umbrella for docs and `cargo test --features full`.

## Tests

`tests/rules.rs` pins the seven discretionary rules from the charter one test
each; `tests/errors.rs` reaches every `FolkError` arm; `tests/projection.rs`
checks the visibility rule on every read; `tests/validate.rs` checks that
every transition preserves validity and that replay is deterministic;
`tests/fold.rs` pins `fold(pre, events) == post` over seeded traces
covering every action arm, accepted and rejected.

## Not yet

- A wasm component build (`cargo component`) of this crate. The contract is
  validated; the guest is not built.
- A second implementation and a conformance run over generated scenarios.
- Index structures. `resolve`, `descendants` and the queries scan; correct
  first, fast when a shell asks.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
