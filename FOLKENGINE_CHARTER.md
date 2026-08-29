# folkengine — a folksonomy domain kernel

*The pure, delivery-agnostic logic of a universal tagging engine: a
poly-hierarchical tag vocabulary, the bindings between tags and opaque items,
and the queries over both — behind a language-neutral, sandbox-enforced
boundary. Status: design + validated WIT contract; no crate yet.*

## The idea in one paragraph

Filesystems organise information by *where it is*: one path, one parent, one
place. A folksonomy organises it by *what it is about*: any number of tags,
applied by any number of people, with the tags themselves forming a vocabulary
that can be shaped over time. folkengine is the kernel of that second model. It
knows nothing about files. It knows about **tags** (a directed acyclic graph of
concepts with preferred labels and aliases), **bindings** (the folksonomy atom,
`(tagger, item, tag)`), and **selections** (which items, as seen by whom, match
a boolean combination of tags). Everything a file manager, a note app, a media
library, a CI system or a poker hand-history archive would do with tags is
either one of those three things — and therefore in the kernel — or it is a
delivery concern and therefore not.

The "programmable ontology" is exactly the vocabulary half of the state: it is
edited through actions, every edit emits an event, and so the ontology's whole
history is replayable. A rename, a merge, or a re-parenting is a first-class
transition, not a migration script.

## Domain in three words

**Tags over items.** If a proposed feature cannot be described as a change to
the vocabulary, a change to the bindings, or a read over the two, it is not
folkengine's job.

## The kernel contract

The full contract is `wit/folkengine.wit` (package
`folkengine:folksonomy@0.1.0`, world `folkengine`, exports one interface,
imports nothing). It resolves under `wasm-tools` and `componentize-py`
generates guest bindings from it, so it is implementable in any component
language today. The surface:

| Function | Kind | What it answers |
|---|---|---|
| `empty(curators)` | constructor | a blank folksonomy with governance set |
| `normalize-label(label)` | pure helper | the one canonical form of a label |
| `validate(state)` | check | is this loaded state one the kernel will accept |
| `apply(state, actor, action)` | **transition** | next state + ordered events, or why not |
| `resolve(state, label)` | read | which tag does this label or alias name |
| `ancestors(state, tag)` / `descendants(state, tag)` | read | the transitive closure in either direction |
| `query(state, actor, selection)` | read | which items match, as this actor sees them |
| `facets(state, actor, selection)` | read | how the matching items could be narrowed further |
| `tags-of(state, actor, item)` | read | what this actor can see on one item |
| `view-for(state, actor)` | **projection** | everything this actor is entitled to see |

State goes in and comes out of every call. The host owns it; the component is
stateless. That is what lets the world import nothing.

Compared with the game-kernel shape (`to-act` / `legal-actions` / `apply` /
`view-for` / `outcome`) two functions are deliberately absent. There is no
`to-act`, because a folksonomy has no turn order — any actor may act at any
time and ordering is the shell's problem. There is no `legal-actions`, because
the action space is unbounded (labels are free text); `apply` returning a
typed `folk-error` is the legality oracle instead. `outcome` has no analogue
because a folksonomy never finishes.

### State

```
folksonomy { next-tag-id, tags, bindings, curators }
tag        { id, label, aliases, parents }
binding    { item, tag, tagger, visibility }
```

Three identities, three different owners. A **tag-id** is allocated by the
kernel from `next-tag-id` — a monotonic counter in state, so allocation is
deterministic and a replayed log yields identical ids without a random source.
An **item-id** is opaque and caller-supplied: the shell decides whether it is a
path, a URL, a content digest, a database key or a poker hand's serial number,
and the kernel compares it bytewise and does nothing else with it. An
**actor-id** is opaque too; it is the "folk" in folksonomy.

Labels are stored normalized. The rule is exported as `normalize-label` so
that shells, UIs and any second implementation apply the same one: trim,
collapse internal whitespace to one space, Unicode lowercase. No Unicode
normalization form is applied — see limits. Every label and alias in the
vocabulary is unique after normalization; `resolve` looks both up.

### Vocabulary: a DAG, not a tree

A tag may have several parents. `rust` can sit under both `languages` and
`systems-programming` without either being a lie, which a tree forces. The
kernel rejects any edit — `add-parent`, `merge-tags`, or a `define-tag` with
parents — that would create a cycle, with `would-cycle`. Transitive reads
(`ancestors`, `descendants`, transitive `tag-ref`s in a selection) are the
reason to have a hierarchy at all: tagging a file `rust` makes it findable
under `languages` without anyone having to tag it twice.

This is deliberately SKOS's shape — a concept with one preferred label, any
number of alternative labels, and broader/narrower relations that may be
poly-hierarchical — with the vocabulary borrowed and the RDF left behind.

### Actions and events

| Action | Open or curated | Events |
|---|---|---|
| `define-tag { label, parents }` | open | `tag-defined` |
| `rename-tag { tag, label }` | curated | `tag-renamed` |
| `add-alias` / `remove-alias { tag, alias }` | curated | `alias-added` / `alias-removed` |
| `add-parent` / `remove-parent { child, parent }` | curated | `parent-added` / `parent-removed` |
| `merge-tags { source, into }` | curated | `parent-*` for every rewired edge, `alias-added` for the absorbed label(s), `untagged`/`tagged` per moved binding, then `tags-merged` |
| `retire-tag(tag)` | curated | `tag-retired` — only if the tag has no bindings and no children |
| `tag { item, tag, visibility }` | open | `tagged`, or `visibility-changed`, or nothing (idempotent) |
| `untag { item, tag, tagger? }` | open for own bindings; curators may name another tagger | `untagged` |

**Events are the product.** They are structured values, never sentences.
A filesystem shell mirrors `tagged` into an xattr or sidecar; a search shell
rebuilds its index from `tags-merged`; a sync shell ships the event log; a
second kernel consumes them as actions (below). Ordering within one `apply` is
part of the contract: a merge emits its constituent rewirings first and the
summary `tags-merged` last, so a consumer that only understands the primitive
events still ends up in the right state.

**Governance is data, not code.** `curators` lives in the state and is set by
the shell that owns the state; the kernel only enforces it. When the list is
empty the vocabulary is fully open, which is the classic del.icio.us shape.
When it is non-empty, *structural* edits require a curator, but `define-tag`,
`tag` and `untag` stay open to everyone — the folk keep tagging, the curators
keep the graph tidy. There is no action to change the curator list: giving the
kernel one would mean inventing a bootstrap rule, and that is a shell decision
with organisational consequences the kernel has no business deciding.

### Selections: a flat query algebra

WIT types cannot recurse, so instead of an expression tree a selection is
disjunctive normal form, flattened:

```
selection { any-of: list<list<tag-ref>>, none-of: list<tag-ref>, taggers: option<list<actor-id>> }
tag-ref   { tag, transitive: bool }
```

An item matches if it satisfies every ref in *some* clause of `any-of` and no
ref in `none-of`. An empty `any-of` is "everything with at least one visible
binding". A ref with `transitive = true` means the tag or any descendant; this
is what makes "show me everything under `languages`" one ref rather than a
query the caller has to expand itself. `taggers` narrows the bindings
considered before evaluation — "things *I* tagged `todo`" versus "things
anyone tagged `todo`".

Results are deduplicated and sorted bytewise by item-id. `facets` counts, over
the matching items, how many distinct items carry each tag *directly*, sorted
by count descending then id ascending. Both orderings are stated because a
second implementation must reproduce them exactly.

Every read takes an actor and applies the visibility rule before evaluating.
That is the point of the next section.

### The hidden-information projection

A binding is `public` or `private`. A private binding is visible only to its
tagger. `view-for(state, actor)` is the single function that encodes that rule
and returns a `folksonomy-view` — deliberately a different type from
`folksonomy`, so a projection cannot be fed back into `apply` by mistake.
Every query, facet and `tags-of` call routes through the same rule.

This is invariant #5 doing real work outside card games. Private tags are how
a shared library supports personal organisation ("mine", "to-read",
"embarrassing draft") without leaking it, and the entitlement decision lives
in exactly one place. The two Facebook failures recorded in
`interlocking-kernels.md` are the reason it is a kernel function and not a
shell filter: a projection implemented as "run the normal read path pretending
to be someone else" carries that path's authority with it; a pure `view-for`
carries nothing but values.

## Named discretionary rules

The kernel does not claim these are the only right answers. It claims that
each is visible, tested and changeable in one place. They are the first things
to pin with tests.

1. **Label normalization** — trim, single-space, Unicode lowercase, no NFC.
2. **Retire requires empty** — no cascading deletes; merge or untag first.
3. **Merge semantics** — bindings move; source label and aliases become
   aliases of the target; parents union; children re-parent; cycle check
   applies to the result. A moved binding that would duplicate an existing
   `(tagger, item, into)` binding is dropped (it emits `untagged` only) and
   is not counted in `moved-bindings`.
4. **Idempotent tagging** — an identical binding emits nothing.
5. **Governance split** — define/tag/untag open, structure curated.
6. **Result orderings** — items bytewise; facets by count desc, id asc.
7. **Transitive default** — hierarchy is consulted only when a ref asks.

## Shells, and what is not a kernel

None of the following lives in folkengine:

**Paths, content and hashing.** A filesystem shell watches directories, maps
each file to an item-id of its own choosing, and mirrors events into
extended attributes or a sidecar file. Whether renaming a file keeps its tags
is the shell's rule about *item identity*, not the kernel's.

**Persistence and sync.** The shell stores state or the event log. Conflict
resolution between two divergent logs is a process domain; if it grows
decisions of its own, it is a process-manager kernel per composition rule 8,
not a feature of this one.

**Full-text search, ranking, recommendation.** A search shell can join its
index against `query` results; "related tags" is a selection plus `facets`,
which is why `facets` is in the kernel and recommendation is not.

**Authentication.** The kernel is told who the actor is. It never finds out.

## Interlocking with other kernels

folkengine is the rare kernel whose *natural* use is as the second party in an
interlock. Values cross; calls don't:

- A pkcore hand-history shell emits `hand-recorded { serial, … }`; a pure
  translator turns it into `tag { item: serial, tag: <resolve "played-2026-08"> }`.
  The tag graph then gives "every hand at this stake level" for free.
- The trucking kernel emits load events; the same shape tags loads by lane,
  shipper and equipment, and a dispatcher's private tags stay theirs.
- Two systems each running their own folkengine state federate by **label,
  not id**: a pure translator maps one vocabulary's labels onto another's via
  `resolve`, because tag-ids are local to one state. That is composition rule 3
  — the contract between two folksonomies is narrower than either surface.

## Positioning

- **vs SKOS** — the same concept/label/broader-narrower data model, but SKOS is
  a data model with no transition function and no tagging triple; folkengine is
  the state machine that edits and queries one, with the RDF and URIs left to a
  shell that wants them.
- **vs the classic folksonomy sites (del.icio.us, Flickr, 2004–)** — the
  tripartite `(user, item, tag)` graph is theirs; the DAG vocabulary, the
  curated/open split, and the private projection are additions they never had,
  and none of them shipped the logic as anything a second system could run.
- **vs OS-level tags (Finder tags, xattrs)** — flat, single-user, per-device,
  bound to a path. folkengine is the part those would share if they were
  designed to.
- **vs nested tags in note apps (Obsidian-style `a/b/c`)** — a tree encoded in
  the label; one parent, no aliases, no merge, rename by search-and-replace.
- **vs semantic file systems (Gifford et al., 1991)** — the same ambition
  (find by attribute, not by path) realised inside the filesystem; folkengine
  keeps the filesystem as one shell among many.
- **vs a graph database** — storage and a query language, not a domain; a
  perfectly good place for a shell to keep folkengine's state.

## Testkit hooks

Per the kernel-testkit pattern, the data textures worth generating first:
deep chains (a 40-level ancestry), wide fans (one parent, thousands of
children), diamonds (the poly-hierarchy case that breaks tree code), near-
collision labels (`Rust`, ` rust `, `RUST`), a public/private mix across many
taggers, and merge chains where the source has its own aliases and children.
State coverage should be measured over the `folk-error` variants as much as
the happy path: every error arm reachable by a generated scenario.

## Honest limits

- **Unicode.** Lowercasing is locale-independent but no normalization form is
  applied, so `é` composed and decomposed are two labels. Adding NFC is a
  deliberate future contract change, and a second implementation must then
  agree on the tables.
- **State-by-value.** The WIT contract passes the whole folksonomy in and out.
  That is the correct *meaning*; for large states the in-process Rust API is
  the performance path and the component boundary is the portability path. A
  handle-based world (resources) is a possible 0.2.
- **Ids are local.** Federation across systems is by label, through a
  translator. Global tag identity is out of scope and probably should stay so.
- **Governance of the contract itself.** Per `POTENTIAL_LIMITS.md`: a change
  policy (what is breaking, on what notice) should ship with 0.1.0 or the
  world is an intention, not a contract.
- **Not yet built.** No crate exists. The WIT resolves and generates bindings;
  nothing has been compiled or run against it.

## Next steps

1. Scaffold the Rust crate against the WIT: `default = []`, zero pure-tree
   dependencies, clippy `disallowed-methods` for the clock and filesystem,
   `check_purity.py` clean, the `kernel-purity` CI job.
2. Pin the seven discretionary rules with tests before anything else.
3. A second implementation of the world (Python via componentize-py is the
   cheap one) and a conformance run over generated scenarios — the check the
   purity gates cannot perform.
4. One independently-owned consumer: a tiny filesystem shell that tags a
   directory and answers `query`, so the boundary gets a caller who could
   have written a `HashMap<Path, Vec<String>>` instead.

## One-line definition

> **folkengine** is the pure logic of a folksonomy — a poly-hierarchical tag
> vocabulary, `(tagger, item, tag)` bindings, and per-actor selections over
> them — behind a no-import WIT world, so that any system can organise
> anything by what it shares rather than where it sits.
