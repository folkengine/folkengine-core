//! The one label-normalization rule (discretionary rule 1).

/// Canonical form of a label: trim, collapse internal whitespace runs to a
/// single ASCII space, Unicode lowercase. No Unicode normalization form is
/// applied — `é` composed and decomposed remain two labels (stated limit).
///
/// Exported so that shells, UIs and second implementations share exactly one
/// rule. An empty result means the label is invalid.
///
/// ```
/// assert_eq!(folkengine::normalize_label("  Systems   Programming "), "systems programming");
/// assert_eq!(folkengine::normalize_label("RUST"), "rust");
/// assert_eq!(folkengine::normalize_label(" \t "), "");
/// ```
#[must_use]
pub fn normalize_label(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    for (i, word) in label.split_whitespace().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.extend(word.chars().flat_map(char::to_lowercase));
    }
    out
}
