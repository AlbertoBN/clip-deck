//! Query parsing helpers and ranking inputs.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryTerm {
    pub text: String,
    pub is_prefix: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankingInputs {
    pub boost_pinned: bool,
}

impl Default for RankingInputs {
    fn default() -> Self {
        Self { boost_pinned: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedQuery {
    pub terms: Vec<QueryTerm>,
    pub ranking: RankingInputs,
}

impl ParsedQuery {
    pub fn terms(&self) -> Vec<&str> {
        self.terms.iter().map(|t| t.text.as_str()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}

/// Parses a raw search string into structured query terms, marking the final
/// term as a prefix-match candidate for incremental search.
pub fn parse_query(input: &str) -> ParsedQuery {
    let words: Vec<&str> = input.split_whitespace().collect();
    let last_index = words.len().saturating_sub(1);
    let terms = words
        .iter()
        .enumerate()
        .map(|(i, word)| QueryTerm { text: word.to_string(), is_prefix: i == last_index && !words.is_empty() })
        .collect();
    ParsedQuery { terms, ranking: RankingInputs::default() }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchFilters {
    pub mime_family: Option<crate::mime::MimeFamily>,
    pub pinned_only: bool,
    pub group_id: Option<String>,
    pub favorite_only: bool,
    pub source_app: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_query_splits_into_terms() {
        let query = parse_query("ssh deploy");
        assert_eq!(query.terms(), vec!["ssh", "deploy"]);
    }

    #[test]
    fn extra_whitespace_does_not_produce_empty_terms() {
        let query = parse_query("  ssh   deploy  ");
        assert_eq!(query.terms(), vec!["ssh", "deploy"]);
    }

    #[test]
    fn final_term_is_flagged_as_a_prefix_term() {
        let query = parse_query("ssh depl");
        assert!(!query.terms[0].is_prefix);
        assert!(query.terms[1].is_prefix);
    }

    #[test]
    fn blank_string_produces_an_explicit_empty_query() {
        assert!(parse_query("").is_empty());
    }

    #[test]
    fn whitespace_only_string_produces_an_explicit_empty_query() {
        assert!(parse_query("   ").is_empty());
    }

    #[test]
    fn filters_can_be_constructed_without_any_text_query() {
        let filters = SearchFilters { pinned_only: true, ..Default::default() };
        assert!(filters.pinned_only);
    }

    #[test]
    fn ranking_inputs_default_to_boosting_pinned_results() {
        let query = parse_query("ssh");
        assert!(query.ranking.boost_pinned);
    }
}
