//! Advanced query parser for search operators.
//!
//! This module provides parsing for advanced search operators:
//! - `site:name` - Restrict search to specific site(s)
//! - `-term` - Exclude results containing term
//! - `"exact phrase"` - Require exact phrase match
//! - `regex:pattern` - Match using regex (advanced)
//! - `|` - Separate multiple query segments (pipe-separated multi-query)

use crate::models::SearchResult;
use regex::Regex;

/// Multi-query container for pipe-separated queries
/// Each segment can have its own site restrictions
#[derive(Debug, Clone, Default)]
pub struct MultiQuery {
    /// Individual query segments (split by |)
    pub segments: Vec<AdvancedQuery>,
    /// Original raw query
    pub raw_query: String,
}

impl MultiQuery {
    /// Parse a multi-query string (pipe-separated segments)
    pub fn parse(input: &str) -> Self {
        let raw_query = input.to_string();
        let segments: Vec<AdvancedQuery> = input
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(AdvancedQuery::parse)
            .collect();

        // If no segments, create one from the full input
        let segments = if segments.is_empty() {
            vec![AdvancedQuery::parse(input)]
        } else {
            segments
        };

        MultiQuery {
            segments,
            raw_query,
        }
    }

    /// Check if this is a simple single-segment query
    pub fn is_single(&self) -> bool {
        self.segments.len() <= 1
    }

    /// Get all unique site restrictions across all segments
    pub fn all_site_restrictions(&self) -> Vec<String> {
        let mut sites: Vec<String> = self
            .segments
            .iter()
            .flat_map(|s| s.site_restrictions.clone())
            .collect();
        sites.sort();
        sites.dedup();
        sites
    }

    /// Get segments that apply to a specific site
    /// Returns segments that either:
    /// 1. Have no site restrictions (apply to all sites)
    /// 2. Explicitly include this site in their site: restrictions
    pub fn segments_for_site(&self, site_name: &str) -> Vec<&AdvancedQuery> {
        let site_lower = site_name.to_lowercase();

        // Check if this site is explicitly mentioned in ANY segment
        let site_mentioned_anywhere = self.segments.iter().any(|seg| {
            seg.site_restrictions
                .iter()
                .any(|s| site_lower.contains(s) || s.contains(&site_lower))
        });

        self.segments
            .iter()
            .filter(|seg| {
                if seg.site_restrictions.is_empty() {
                    // Segment has no site restriction - applies to all sites
                    true
                } else {
                    // Segment has site restrictions - check if this site matches
                    seg.site_restrictions
                        .iter()
                        .any(|s| site_lower.contains(s) || s.contains(&site_lower))
                }
            })
            .collect()
    }

    /// Get the combined search terms for a specific site (for URL building)
    /// Returns terms from all applicable segments
    pub fn get_search_terms_for_site(&self, site_name: &str) -> Vec<String> {
        self.segments_for_site(site_name)
            .iter()
            .map(|seg| seg.get_search_terms())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Filter results for a specific site using applicable segments
    pub fn filter_results_for_site(
        &self,
        results: Vec<SearchResult>,
        site_name: &str,
    ) -> Vec<SearchResult> {
        let applicable_segments = self.segments_for_site(site_name);

        if applicable_segments.is_empty() {
            return results;
        }

        // A result matches if it matches ANY applicable segment
        results
            .into_iter()
            .filter(|result| {
                applicable_segments
                    .iter()
                    .any(|seg| seg.matches_result(result))
            })
            .collect()
    }

    /// Check if the multi-query is empty
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() || self.segments.iter().all(|s| s.is_empty())
    }

    /// Get the first segment (for backward compatibility)
    pub fn first(&self) -> Option<&AdvancedQuery> {
        self.segments.first()
    }
}

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

            // Site restriction: site:name or site:name1,name2,name3
            if let Some(site) = token.strip_prefix("site:") {
                if !site.is_empty() {
                    // Support comma-separated sites: site:fitgirl,dodi,elamigos
                    for s in site.split(',') {
                        let s = s.trim();
                        if !s.is_empty() {
                            query.site_restrictions.push(s.to_lowercase());
                        }
                    }
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
  |             Separate multiple queries (each can have own site: filter)

Examples:
  elden ring site:fitgirl
  elden ring -deluxe -edition
  "elden ring" site:dodi
  cyberpunk regex:v[0-9]+\.[0-9]+

Multi-Query Examples:
  elden ring site:fitgirl | minecraft site:csrin
    - Searches fitgirl for "elden ring" AND csrin for "minecraft"
  
  elden ring -nightreign site:fitgirl,dodi | minecraft site:elamigos,csrin
    - Searches fitgirl,dodi for "elden ring" (excluding nightreign)
    - Searches elamigos,csrin for "minecraft"
    - Unmentioned sites search for BOTH queries"#
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
    fn test_parse_comma_separated_sites() {
        let query = AdvancedQuery::parse("minecraft site:fitgirl,dodi,elamigos");
        assert_eq!(query.terms, vec!["minecraft"]);
        assert_eq!(query.site_restrictions, vec!["fitgirl", "dodi", "elamigos"]);
    }

    #[test]
    fn test_parse_comma_and_separate_sites() {
        // Mix of comma-separated and separate site: operators
        let query = AdvancedQuery::parse("game site:fitgirl,dodi site:steamrip");
        assert_eq!(query.site_restrictions, vec!["fitgirl", "dodi", "steamrip"]);
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

    // MultiQuery tests
    #[test]
    fn test_multi_query_single_segment() {
        let mq = MultiQuery::parse("elden ring site:fitgirl");
        assert!(mq.is_single());
        assert_eq!(mq.segments.len(), 1);
        assert_eq!(mq.segments[0].terms, vec!["elden", "ring"]);
    }

    #[test]
    fn test_multi_query_two_segments() {
        let mq = MultiQuery::parse("elden ring site:fitgirl | minecraft site:csrin");
        assert!(!mq.is_single());
        assert_eq!(mq.segments.len(), 2);
        assert_eq!(mq.segments[0].terms, vec!["elden", "ring"]);
        assert_eq!(mq.segments[0].site_restrictions, vec!["fitgirl"]);
        assert_eq!(mq.segments[1].terms, vec!["minecraft"]);
        assert_eq!(mq.segments[1].site_restrictions, vec!["csrin"]);
    }

    #[test]
    fn test_multi_query_segments_for_site() {
        let mq = MultiQuery::parse("elden ring site:fitgirl | minecraft site:csrin | cyberpunk");

        // fitgirl should get "elden ring" segment + "cyberpunk" (no site restriction)
        let fitgirl_segs = mq.segments_for_site("fitgirl");
        assert_eq!(fitgirl_segs.len(), 2);

        // csrin should get "minecraft" segment + "cyberpunk" (no site restriction)
        let csrin_segs = mq.segments_for_site("csrin");
        assert_eq!(csrin_segs.len(), 2);

        // dodi is not mentioned, so gets only "cyberpunk" (no site restriction)
        let dodi_segs = mq.segments_for_site("dodi");
        assert_eq!(dodi_segs.len(), 1);
        assert_eq!(dodi_segs[0].terms, vec!["cyberpunk"]);
    }

    #[test]
    fn test_multi_query_search_terms_for_site() {
        let mq = MultiQuery::parse("elden ring site:fitgirl | minecraft site:csrin");

        let fitgirl_terms = mq.get_search_terms_for_site("fitgirl");
        assert_eq!(fitgirl_terms, vec!["elden ring"]);

        let csrin_terms = mq.get_search_terms_for_site("csrin");
        assert_eq!(csrin_terms, vec!["minecraft"]);
    }

    #[test]
    fn test_multi_query_unspecified_site_gets_all() {
        let mq = MultiQuery::parse("elden ring | minecraft");

        // Both segments have no site restrictions, so any site gets both
        let any_site_segs = mq.segments_for_site("steamrip");
        assert_eq!(any_site_segs.len(), 2);

        let terms = mq.get_search_terms_for_site("steamrip");
        assert!(terms.contains(&"elden ring".to_string()));
        assert!(terms.contains(&"minecraft".to_string()));
    }

    #[test]
    fn test_multi_query_filter_results() {
        let mq = MultiQuery::parse("elden ring site:fitgirl | minecraft site:csrin");

        let results = vec![
            make_result("fitgirl", "Elden Ring", "https://f.com/1"),
            make_result("fitgirl", "Minecraft", "https://f.com/2"),
            make_result("csrin", "Elden Ring", "https://c.com/1"),
            make_result("csrin", "Minecraft", "https://c.com/2"),
        ];

        // For fitgirl, only "elden ring" segment applies
        let fitgirl_filtered = mq.filter_results_for_site(results.clone(), "fitgirl");
        assert_eq!(fitgirl_filtered.len(), 1);
        assert!(fitgirl_filtered[0].title.contains("Elden Ring"));

        // For csrin, only "minecraft" segment applies
        let csrin_filtered = mq.filter_results_for_site(results, "csrin");
        assert_eq!(csrin_filtered.len(), 1);
        assert!(csrin_filtered[0].title.contains("Minecraft"));
    }

    #[test]
    fn test_multi_query_all_site_restrictions() {
        let mq = MultiQuery::parse("elden ring site:fitgirl,dodi | minecraft site:csrin,elamigos");
        let all_sites = mq.all_site_restrictions();
        assert!(all_sites.contains(&"fitgirl".to_string()));
        assert!(all_sites.contains(&"dodi".to_string()));
        assert!(all_sites.contains(&"csrin".to_string()));
        assert!(all_sites.contains(&"elamigos".to_string()));
    }

    #[test]
    fn test_multi_query_empty() {
        let mq = MultiQuery::parse("");
        assert!(mq.is_empty());
    }

    #[test]
    fn test_multi_query_operator_help_contains_pipe() {
        let help = operator_help();
        assert!(help.contains("|"));
        assert!(help.contains("Multi-Query"));
    }
}
