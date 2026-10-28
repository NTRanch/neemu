//! Canonical minor-coded blocklist — a faithful Rust port of the legal
//! scraper's `BLOCKLIST_TERMS` + `check_blocklist`
//! (`~/workspace/legal-nsfw-scraper-20261025/legal_scraper.py`, mirrored by
//! that project's own Rust port in `src/lib.rs`).
//!
//! Neemu SPEC §2.1: the screen reuses the canonical blocklist "imported,
//! never forked" as its floor. The terms, their order, and the matching
//! semantics are therefore copied verbatim from the canonical source:
//! the haystack is the lowercased title followed by each tag, joined with
//! single spaces; matching is by substring; the first term (in list order)
//! found in the haystack is the reported hit.

/// The canonical blocklist terms, in canonical order. Do not edit locally:
/// changes belong in the legal scraper, the canonical home.
pub const BLOCKLIST_TERMS: [&str; 12] = [
    "child",
    "children",
    "minor",
    "underage",
    "loli",
    "shota",
    "schoolgirl",
    "schoolboy",
    "teen",
    "kid",
    "young girl",
    "young boy",
];

/// Port of `check_blocklist`: returns the first blocklist term found in the
/// candidate's text metadata, or `None` when the metadata is clean.
///
/// The haystack is `title` + `" "` + each tag in order, all lowercased —
/// exactly the Python `" ".join([title, *tags]).lower()`.
pub fn blocklist_hit(title: &str, tags: &[String]) -> Option<&'static str> {
    let mut haystack = title.to_string();
    for tag in tags {
        haystack.push(' ');
        haystack.push_str(tag);
    }
    let haystack = haystack.to_lowercase();
    BLOCKLIST_TERMS
        .iter()
        .find(|term| haystack.contains(**term))
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_metadata_passes() {
        assert_eq!(
            blocklist_hit("Moonlit Wife", &["hypnosis".to_string()]),
            None
        );
    }

    #[test]
    fn term_in_title_hits() {
        assert_eq!(blocklist_hit("Schoolgirl Diary", &[]), Some("schoolgirl"));
    }

    #[test]
    fn term_in_tag_hits_case_insensitively() {
        assert_eq!(
            blocklist_hit("Clean Title", &["LoLi".to_string()]),
            Some("loli")
        );
    }

    #[test]
    fn first_term_in_list_order_wins() {
        // "kid" appears in the text before "loli" does, but "loli" precedes
        // "kid" in BLOCKLIST_TERMS order, and the canonical check iterates
        // terms, not text positions.
        assert_eq!(blocklist_hit("kid meets loli", &[]), Some("loli"));
    }
}
