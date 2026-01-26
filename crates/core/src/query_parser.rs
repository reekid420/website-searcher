//! Advanced query parser for search operators.
//!
//! This module provides parsing for advanced search operators:
//! - `site:name` - Restrict search to specific site(s)
//! - `-term` - Exclude results containing term
//! - `"exact phrase"` - Require exact phrase match
//! - `regex:pattern` - Match using regex (advanced)

use crate::models::SearchResult;
use regex::Regex;

/// Parsed advanced query with operator support
#[derive(Debug, Clone, Default)]
pub struct AdvancedQuery {
    /// Regular search terms
    pub terms: Vec<String>,
    /// Terms to exclude (prefixed with -)
    pub exclude_terms: Vec<String>,
    /// Site restrictions (site:name)
    pub site_restrictions: Vec<String>,
    /// Exact phrases (quoted)
    pub exact_phrases: Vec<String>,
    /// Regex patterns (regex:pattern)
    pub regex_patterns: Vec<Regex>,
    /// Original raw query
    pub raw_query: String,
}

impl AdvancedQuery {
    /// Parse a query string into an AdvancedQuery
    pub fn parse(input: &str) -> Self {
        let mut query = AdvancedQuery {
            raw_query: input.to_string(),
            ..Default::default()
        };

        let input = input.trim();
        if input.is_empty() {
            return query;
        }

        // Extract quoted phrases first
        let mut remaining = input.to_string();
        let quote_regex = Regex::new(r#""([^"]+)""#).unwrap();

        for cap in quote_regex.captures_iter(input) {
            if let Some(phrase) = cap.get(1) {
                query.exact_phrases.push(phrase.as_str().to_string());
            }
        }

        // Remove quoted sections from remaining
        remaining = quote_regex.replace_all(&remaining, " ").to_string();

        // Parse remaining tokens
        for token in remaining.split_whitespace() {
            if token.is_empty() {
                continue;
            }

            // Site restriction: site:name
            if let Some(site) = token.strip_prefix("site:") {
                if !site.is_empty() {
                    query.site_restrictions.push(site.to_lowercase());
                }
                continue;
            }

            // Regex pattern: regex:pattern
            if let Some(pattern) = token.strip_prefix("regex:") {
                if !pattern.is_empty()
                    && let Ok(re) = Regex::new(pattern)
                {
                    query.regex_patterns.push(re);
                }
                continue;
            }

            // Exclusion: -term
            if let Some(excluded) = token.strip_prefix('-') {
                if !excluded.is_empty() {
                    query.exclude_terms.push(excluded.to_lowercase());
                }
                continue;
            }

            // Regular term
            query.terms.push(token.to_string());
        }

        query
    }

    /// Get the search terms as a single string for URL building
    pub fn get_search_terms(&self) -> String {
        let mut terms = self.terms.clone();

        // Include exact phrases in search
        for phrase in &self.exact_phrases {
            terms.push(phrase.clone());
        }

        terms.join(" ")
    }

    /// Check if a search result matches this query's filters
    pub fn matches_result(&self, result: &SearchResult) -> bool {
        let title_lower = result.title.to_lowercase();
        let url_lower = result.url.to_lowercase();

        // Check site restrictions
        if !self.site_restrictions.is_empty() {
            let site_lower = result.site.to_lowercase();
            if !self
                .site_restrictions
                .iter()
                .any(|s| site_lower.contains(s))
            {
                return false;
            }
        }

        // Check exclusions
        for excluded in &self.exclude_terms {
            if title_lower.contains(excluded) || url_lower.contains(excluded) {
                return false;
            }
        }

        // Check exact phrases
        for phrase in &self.exact_phrases {
            let phrase_lower = phrase.to_lowercase();
            if !title_lower.contains(&phrase_lower) && !url_lower.contains(&phrase_lower) {
                return false;
            }
        }

        // Check regex patterns
        for pattern in &self.regex_patterns {
            if !pattern.is_match(&result.title) && !pattern.is_match(&result.url) {
                return false;
            }
        }

        true
    }

    /// Get site filter list if any restrictions are present
    pub fn get_sites_filter(&self) -> Option<Vec<String>> {
        if self.site_restrictions.is_empty() {
            None
        } else {
            Some(self.site_restrictions.clone())
        }
    }

    /// Check if the query has any advanced operators
    pub fn has_operators(&self) -> bool {
        !self.exclude_terms.is_empty()
            || !self.site_restrictions.is_empty()
            || !self.exact_phrases.is_empty()
            || !self.regex_patterns.is_empty()
    }

    /// Check if the query is empty
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty() && self.exact_phrases.is_empty()
    }

    /// Get the original raw query
    pub fn raw(&self) -> &str {
        &self.raw_query
    }
}

/// Filter a list of results using the advanced query
pub fn filter_results(results: Vec<SearchResult>, query: &AdvancedQuery) -> Vec<SearchResult> {
    if !query.has_operators() {
        return results;
    }

    results
        .into_iter()
        .filter(|r| query.matches_result(r))
        .collect()
}

/// Parse a query and extract just the search terms (backward compatible)
pub fn extract_search_terms(input: &str) -> String {
    let query = AdvancedQuery::parse(input);
    query.get_search_terms()
}

/// Help text for advanced query operators
pub fn operator_help() -> &'static str {
    r#"Advanced Query Operators:
  site:name     Restrict to specific site (e.g., site:fitgirl)
  -term         Exclude results containing term (e.g., -deluxe)
  "phrase"      Require exact phrase match (e.g., "elden ring")
  regex:pattern Match using regex (e.g., regex:v[0-9]+)

Examples:
  elden ring site:fitgirl
  elden ring -deluxe -edition
  "elden ring" site:dodi
  cyberpunk regex:v[0-9]+\.[0-9]+"#
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(site: &str, title: &str, url: &str) -> SearchResult {
        SearchResult {
            site: site.to_string(),
            title: title.to_string(),
            url: url.to_string(),
        }
    }

    #[test]
    fn test_parse_simple_query() {
        let query = AdvancedQuery::parse("elden ring");
        assert_eq!(query.terms, vec!["elden", "ring"]);
        assert!(query.exclude_terms.is_empty());
        assert!(query.site_restrictions.is_empty());
        assert!(query.exact_phrases.is_empty());
    }

    #[test]
    fn test_parse_site_operator() {
        let query = AdvancedQuery::parse("elden ring site:fitgirl");
        assert_eq!(query.terms, vec!["elden", "ring"]);
        assert_eq!(query.site_restrictions, vec!["fitgirl"]);
    }

    #[test]
    fn test_parse_multiple_sites() {
        let query = AdvancedQuery::parse("elden ring site:fitgirl site:dodi");
        assert_eq!(query.site_restrictions, vec!["fitgirl", "dodi"]);
    }

    #[test]
    fn test_parse_exclude_operator() {
        let query = AdvancedQuery::parse("elden ring -deluxe -edition");
        assert_eq!(query.terms, vec!["elden", "ring"]);
        assert_eq!(query.exclude_terms, vec!["deluxe", "edition"]);
    }

    #[test]
    fn test_parse_exact_phrase() {
        let query = AdvancedQuery::parse(r#""elden ring" dlc"#);
        assert_eq!(query.exact_phrases, vec!["elden ring"]);
        assert_eq!(query.terms, vec!["dlc"]);
    }

    #[test]
    fn test_parse_multiple_phrases() {
        let query = AdvancedQuery::parse(r#""elden ring" "shadow of the erdtree""#);
        assert_eq!(query.exact_phrases.len(), 2);
        assert!(query.exact_phrases.contains(&"elden ring".to_string()));
        assert!(
            query
                .exact_phrases
                .contains(&"shadow of the erdtree".to_string())
        );
    }

    #[test]
    fn test_parse_regex_operator() {
        let query = AdvancedQuery::parse("game regex:v[0-9]+");
        assert_eq!(query.terms, vec!["game"]);
        assert_eq!(query.regex_patterns.len(), 1);
        assert!(query.regex_patterns[0].is_match("v123"));
    }

    #[test]
    fn test_parse_combined_operators() {
        let query = AdvancedQuery::parse(r#""elden ring" site:fitgirl -deluxe"#);
        assert_eq!(query.exact_phrases, vec!["elden ring"]);
        assert_eq!(query.site_restrictions, vec!["fitgirl"]);
        assert_eq!(query.exclude_terms, vec!["deluxe"]);
    }

    #[test]
    fn test_get_search_terms() {
        let query = AdvancedQuery::parse(r#"elden ring site:fitgirl -deluxe"#);
        assert_eq!(query.get_search_terms(), "elden ring");
    }

    #[test]
    fn test_get_search_terms_with_phrase() {
        let query = AdvancedQuery::parse(r#""elden ring" dlc"#);
        let terms = query.get_search_terms();
        assert!(terms.contains("dlc"));
        assert!(terms.contains("elden ring"));
    }

    #[test]
    fn test_matches_result_site_restriction() {
        let query = AdvancedQuery::parse("elden ring site:fitgirl");

        let match_result = make_result("fitgirl", "Elden Ring", "https://example.com");
        let no_match = make_result("dodi", "Elden Ring", "https://example.com");

        assert!(query.matches_result(&match_result));
        assert!(!query.matches_result(&no_match));
    }

    #[test]
    fn test_matches_result_exclusion() {
        let query = AdvancedQuery::parse("elden ring -deluxe");

        let match_result = make_result("fitgirl", "Elden Ring", "https://example.com");
        let no_match = make_result(
            "fitgirl",
            "Elden Ring Deluxe Edition",
            "https://example.com",
        );

        assert!(query.matches_result(&match_result));
        assert!(!query.matches_result(&no_match));
    }

    #[test]
    fn test_matches_result_exact_phrase() {
        let query = AdvancedQuery::parse(r#""elden ring""#);

        let match_result = make_result("fitgirl", "Elden Ring - Full Game", "https://example.com");
        let no_match = make_result("fitgirl", "Elden's Ring", "https://example.com");

        assert!(query.matches_result(&match_result));
        assert!(!query.matches_result(&no_match));
    }

    #[test]
    fn test_matches_result_regex() {
        let query = AdvancedQuery::parse("game regex:v[0-9]+\\.[0-9]+");

        let match_result = make_result("fitgirl", "Game v1.5", "https://example.com");
        let no_match = make_result("fitgirl", "Game", "https://example.com");

        assert!(query.matches_result(&match_result));
        assert!(!query.matches_result(&no_match));
    }

    #[test]
    fn test_filter_results() {
        let query = AdvancedQuery::parse("elden ring site:fitgirl -deluxe");

        let results = vec![
            make_result("fitgirl", "Elden Ring", "https://f.com/1"),
            make_result("dodi", "Elden Ring", "https://d.com/1"),
            make_result("fitgirl", "Elden Ring Deluxe", "https://f.com/2"),
        ];

        let filtered = filter_results(results, &query);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].url, "https://f.com/1");
    }

    #[test]
    fn test_has_operators() {
        let simple = AdvancedQuery::parse("elden ring");
        assert!(!simple.has_operators());

        let with_site = AdvancedQuery::parse("elden ring site:fitgirl");
        assert!(with_site.has_operators());

        let with_exclude = AdvancedQuery::parse("elden ring -deluxe");
        assert!(with_exclude.has_operators());
    }

    #[test]
    fn test_is_empty() {
        let empty = AdvancedQuery::parse("");
        assert!(empty.is_empty());

        let not_empty = AdvancedQuery::parse("elden ring");
        assert!(!not_empty.is_empty());

        let phrase_only = AdvancedQuery::parse(r#""elden ring""#);
        assert!(!phrase_only.is_empty());
    }

    #[test]
    fn test_get_sites_filter() {
        let no_sites = AdvancedQuery::parse("elden ring");
        assert!(no_sites.get_sites_filter().is_none());

        let with_sites = AdvancedQuery::parse("elden ring site:fitgirl site:dodi");
        let sites = with_sites.get_sites_filter().unwrap();
        assert_eq!(sites.len(), 2);
    }

    #[test]
    fn test_empty_query() {
        let query = AdvancedQuery::parse("");
        assert!(query.terms.is_empty());
        assert!(query.is_empty());
    }

    #[test]
    fn test_whitespace_only_query() {
        let query = AdvancedQuery::parse("   \t\n  ");
        assert!(query.terms.is_empty());
        assert!(query.is_empty());
    }

    #[test]
    fn test_extract_search_terms() {
        let terms = extract_search_terms("elden ring site:fitgirl -deluxe");
        assert_eq!(terms, "elden ring");
    }

    #[test]
    fn test_case_insensitive_site_match() {
        let query = AdvancedQuery::parse("game site:FitGirl");
        let result = make_result("FITGIRL", "Game", "https://example.com");
        assert!(query.matches_result(&result));
    }

    #[test]
    fn test_case_insensitive_exclusion() {
        let query = AdvancedQuery::parse("game -DELUXE");
        let result = make_result("fitgirl", "Game Deluxe", "https://example.com");
        assert!(!query.matches_result(&result));
    }

    #[test]
    fn test_invalid_regex_ignored() {
        let query = AdvancedQuery::parse("game regex:[invalid(");
        // Invalid regex should be ignored, not cause panic
        assert!(query.regex_patterns.is_empty());
        assert_eq!(query.terms, vec!["game"]);
    }

    #[test]
    fn test_operator_help() {
        let help = operator_help();
        assert!(help.contains("site:"));
        assert!(help.contains("-term"));
        assert!(help.contains("regex:"));
    }

    #[test]
    fn test_exclusion_in_url() {
        let query = AdvancedQuery::parse("game -deluxe");
        let result = make_result("fitgirl", "Game", "https://example.com/deluxe-edition");
        assert!(!query.matches_result(&result));
    }
}
