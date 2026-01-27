//! Content analyzer module for metadata extraction and duplicate detection.
//!
//! This module provides:
//! - Metadata extraction from titles (file sizes, versions, dates)
//! - Title similarity scoring using Levenshtein distance
//! - Cross-site duplicate detection

use crate::models::SearchResult;
use serde::{Deserialize, Serialize};

/// Extracted metadata from a search result title
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResultMetadata {
    /// File size (e.g., "45.2 GB")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<String>,
    /// Release or update date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    /// Version string (e.g., "v1.2.3")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Build number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
}

impl ResultMetadata {
    /// Check if any metadata was extracted
    pub fn has_data(&self) -> bool {
        self.file_size.is_some()
            || self.release_date.is_some()
            || self.version.is_some()
            || self.build.is_some()
    }
}

/// Content analyzer for result processing
#[derive(Debug, Clone)]
pub struct ContentAnalyzer {
    /// Similarity threshold for duplicate detection (0.0-1.0)
    pub duplicate_threshold: f32,
}

impl Default for ContentAnalyzer {
    fn default() -> Self {
        Self {
            duplicate_threshold: 0.85,
        }
    }
}

impl ContentAnalyzer {
    /// Create a new content analyzer
    pub fn new() -> Self {
        Self::default()
    }

    /// Create analyzer with custom duplicate threshold
    pub fn with_threshold(threshold: f32) -> Self {
        Self {
            duplicate_threshold: threshold.clamp(0.0, 1.0),
        }
    }

    /// Extract metadata from a title string
    pub fn extract_metadata(&self, title: &str) -> ResultMetadata {
        extract_metadata(title)
    }

    /// Calculate similarity between two titles
    pub fn calculate_similarity(&self, a: &str, b: &str) -> f32 {
        calculate_similarity(a, b)
    }

    /// Find duplicate pairs in a list of results
    pub fn find_duplicates(&self, results: &[SearchResult]) -> Vec<(usize, usize)> {
        find_duplicates_with_threshold(results, self.duplicate_threshold)
    }

    /// Remove duplicates from results, keeping the first occurrence
    pub fn deduplicate_results(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        deduplicate_results_with_threshold(results, self.duplicate_threshold)
    }
}

/// Extract metadata from a title string
pub fn extract_metadata(title: &str) -> ResultMetadata {
    let mut metadata = ResultMetadata::default();

    // Extract file size (e.g., "45.2 GB", "12.5 MB", "1.2TB")
    let size_patterns = [
        r"[\[(]?\s*(\d+(?:\.\d+)?\s*(?:GB|MB|TB|GiB|MiB|TiB))\s*[\])]?",
        r"[\|(](\d+(?:\.\d+)?\s*(?:GB|MB|TB))[)\]]?",
    ];

    for pattern in size_patterns {
        if let Ok(re) = regex::Regex::new(&format!("(?i){}", pattern))
            && let Some(cap) = re.captures(title)
            && let Some(size) = cap.get(1)
        {
            metadata.file_size = Some(size.as_str().to_uppercase().replace(" ", ""));
            break;
        }
    }

    // Extract version (e.g., "v1.2.3", "Version 1.0", "1.2.3.4")
    let version_patterns = [
        r"[vV](\d+\.\d+(?:\.\d+)*)",
        r"[vV]ersion\s+(\d+\.\d+(?:\.\d+)*)",
        r"\[(\d+\.\d+\.\d+(?:\.\d+)?)\]",
    ];

    for pattern in version_patterns {
        if let Ok(re) = regex::Regex::new(&format!("(?i){}", pattern))
            && let Some(cap) = re.captures(title)
            && let Some(ver) = cap.get(1)
        {
            metadata.version = Some(format!("v{}", ver.as_str()));
            break;
        }
    }

    // Extract build number (e.g., "Build 12345", "b12345")
    if let Ok(re) = regex::Regex::new(r"(?i)(?:build\s*|b)(\d{4,})")
        && let Some(cap) = re.captures(title)
        && let Some(build) = cap.get(1)
    {
        metadata.build = Some(build.as_str().to_string());
    }

    // Extract date (e.g., "2024-01-15", "01/15/2024", "15.01.2024")
    let date_patterns = [
        r"(\d{4}[-/]\d{2}[-/]\d{2})", // YYYY-MM-DD
        r"(\d{2}[-/]\d{2}[-/]\d{4})", // DD-MM-YYYY or MM-DD-YYYY
        r"(\d{2}\.\d{2}\.\d{4})",     // DD.MM.YYYY
    ];

    for pattern in date_patterns {
        if let Ok(re) = regex::Regex::new(pattern)
            && let Some(cap) = re.captures(title)
            && let Some(date) = cap.get(1)
        {
            metadata.release_date = Some(date.as_str().to_string());
            break;
        }
    }

    metadata
}

/// Calculate Levenshtein similarity between two strings (0.0 to 1.0)
pub fn calculate_similarity(a: &str, b: &str) -> f32 {
    let a_normalized = normalize_for_comparison(a);
    let b_normalized = normalize_for_comparison(b);

    if a_normalized.is_empty() && b_normalized.is_empty() {
        return 1.0;
    }

    if a_normalized.is_empty() || b_normalized.is_empty() {
        return 0.0;
    }

    let distance = levenshtein_distance(&a_normalized, &b_normalized);
    let max_len = a_normalized.len().max(b_normalized.len());

    1.0 - (distance as f32 / max_len as f32)
}

/// Normalize a title for comparison
fn normalize_for_comparison(title: &str) -> String {
    let mut normalized = title.to_lowercase();

    // Remove common noise patterns
    let noise_patterns = [
        r"\s*[\[(][^\])]*(?:gb|mb|tb|gib|mib|tib)[\])]", // Size markers in brackets
        r"\s*[\[(]v?\d+(?:\.\d+)+[\])]",                 // Version markers in brackets
        r"\s*v\d+(?:\.\d+)+",        // Standalone version markers (e.g., v1.2.3)
        r"\s*[\[(]build\s*\d+[\])]", // Build markers
        r"(?:repack|rip|proper|update|fix)", // Release tags
        r"[-_]+",                    // Separators
    ];

    for pattern in noise_patterns {
        if let Ok(re) = regex::Regex::new(&format!("(?i){}", pattern)) {
            normalized = re.replace_all(&normalized, " ").to_string();
        }
    }

    // Collapse whitespace
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    parts.join(" ")
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Create two rows for the DP table
    let mut prev_row: Vec<usize> = (0..=n).collect();
    let mut curr_row: Vec<usize> = vec![0; n + 1];

    for i in 1..=m {
        curr_row[0] = i;

        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr_row[j] = (prev_row[j] + 1)
                .min(curr_row[j - 1] + 1)
                .min(prev_row[j - 1] + cost);
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[n]
}

/// Find duplicate pairs based on title similarity
pub fn find_duplicates_with_threshold(
    results: &[SearchResult],
    threshold: f32,
) -> Vec<(usize, usize)> {
    let mut duplicates = Vec::new();

    for i in 0..results.len() {
        for j in (i + 1)..results.len() {
            // Skip if same site (not cross-site duplicate)
            if results[i].site == results[j].site {
                continue;
            }

            let similarity = calculate_similarity(&results[i].title, &results[j].title);
            if similarity >= threshold {
                duplicates.push((i, j));
            }
        }
    }

    duplicates
}

/// Remove cross-site duplicates, keeping the first occurrence
pub fn deduplicate_results_with_threshold(
    results: Vec<SearchResult>,
    threshold: f32,
) -> Vec<SearchResult> {
    if results.is_empty() {
        return results;
    }

    let mut keep = vec![true; results.len()];
    let duplicates = find_duplicates_with_threshold(&results, threshold);

    // Mark later duplicates for removal
    for (_, j) in duplicates {
        keep[j] = false;
    }

    results
        .into_iter()
        .enumerate()
        .filter_map(|(i, r)| if keep[i] { Some(r) } else { None })
        .collect()
}

/// Deduplicate results using default threshold (0.95 for strict matching)
pub fn deduplicate_results(results: Vec<SearchResult>) -> Vec<SearchResult> {
    deduplicate_results_with_threshold(results, 0.95)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(site: &str, title: &str) -> SearchResult {
        SearchResult {
            site: site.to_string(),
            title: title.to_string(),
            url: format!("https://{}.com/test", site),
        }
    }

    #[test]
    fn test_extract_file_size_gb() {
        let meta = extract_metadata("Game Name [45.2 GB]");
        assert_eq!(meta.file_size, Some("45.2GB".to_string()));
    }

    #[test]
    fn test_extract_file_size_mb() {
        let meta = extract_metadata("Game Name (500 MB)");
        assert_eq!(meta.file_size, Some("500MB".to_string()));
    }

    #[test]
    fn test_extract_file_size_tb() {
        let meta = extract_metadata("Game Name | 1.2 TB");
        assert_eq!(meta.file_size, Some("1.2TB".to_string()));
    }

    #[test]
    fn test_extract_version_v_prefix() {
        let meta = extract_metadata("Game Name v1.2.3");
        assert_eq!(meta.version, Some("v1.2.3".to_string()));
    }

    #[test]
    fn test_extract_version_in_brackets() {
        let meta = extract_metadata("Game Name [1.2.3.4]");
        assert_eq!(meta.version, Some("v1.2.3.4".to_string()));
    }

    #[test]
    fn test_extract_build_number() {
        let meta = extract_metadata("Game Name Build 12345");
        assert_eq!(meta.build, Some("12345".to_string()));
    }

    #[test]
    fn test_extract_date_iso() {
        let meta = extract_metadata("Game Name 2024-01-15");
        assert_eq!(meta.release_date, Some("2024-01-15".to_string()));
    }

    #[test]
    fn test_extract_multiple_metadata() {
        let meta = extract_metadata("Game Name v1.5.2 [45 GB] Build 12345");
        assert_eq!(meta.version, Some("v1.5.2".to_string()));
        assert_eq!(meta.file_size, Some("45GB".to_string()));
        assert_eq!(meta.build, Some("12345".to_string()));
    }

    #[test]
    fn test_extract_no_metadata() {
        let meta = extract_metadata("Just A Game Name");
        assert!(!meta.has_data());
    }

    #[test]
    fn test_similarity_identical() {
        let sim = calculate_similarity("Elden Ring", "Elden Ring");
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_similarity_similar() {
        let sim = calculate_similarity("Elden Ring", "Elden Ring [45 GB]");
        assert!(sim > 0.8);
    }

    #[test]
    fn test_similarity_different() {
        let sim = calculate_similarity("Elden Ring", "Cyberpunk 2077");
        assert!(sim < 0.5);
    }

    #[test]
    fn test_similarity_empty_strings() {
        assert!((calculate_similarity("", "") - 1.0).abs() < 0.01);
        assert!((calculate_similarity("test", "") - 0.0).abs() < 0.01);
        assert!((calculate_similarity("", "test") - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_find_duplicates_cross_site() {
        let results = vec![
            make_result("fitgirl", "Elden Ring"),
            make_result("dodi", "Elden Ring"),
            make_result("steamrip", "Cyberpunk 2077"),
        ];

        let duplicates = find_duplicates_with_threshold(&results, 0.85);
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0], (0, 1));
    }

    #[test]
    fn test_find_duplicates_same_site_ignored() {
        let results = vec![
            make_result("fitgirl", "Elden Ring"),
            make_result("fitgirl", "Elden Ring DLC"),
        ];

        let duplicates = find_duplicates_with_threshold(&results, 0.85);
        assert!(duplicates.is_empty());
    }

    #[test]
    fn test_deduplicate_results() {
        let results = vec![
            make_result("fitgirl", "Elden Ring"),
            make_result("dodi", "Elden Ring"),
            make_result("steamrip", "Cyberpunk 2077"),
            make_result("gog", "Cyberpunk 2077"),
        ];

        let deduped = deduplicate_results(results);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].site, "fitgirl");
        assert_eq!(deduped[1].site, "steamrip");
    }

    #[test]
    fn test_deduplicate_empty() {
        let results: Vec<SearchResult> = vec![];
        let deduped = deduplicate_results(results);
        assert!(deduped.is_empty());
    }

    #[test]
    fn test_content_analyzer_builder() {
        let analyzer = ContentAnalyzer::with_threshold(0.9);
        assert!((analyzer.duplicate_threshold - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_content_analyzer_extract() {
        let analyzer = ContentAnalyzer::new();
        let meta = analyzer.extract_metadata("Game v1.0 [10 GB]");
        assert!(meta.has_data());
    }

    #[test]
    fn test_metadata_has_data() {
        let empty = ResultMetadata::default();
        assert!(!empty.has_data());

        let with_size = ResultMetadata {
            file_size: Some("10GB".to_string()),
            ..Default::default()
        };
        assert!(with_size.has_data());
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "abd"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_normalize_for_comparison() {
        let normalized = normalize_for_comparison("Game Name [45 GB] v1.2.3");
        assert!(!normalized.contains("gb"));
        assert!(!normalized.contains("1.2.3"));
    }

    #[test]
    fn test_similarity_case_insensitive() {
        let sim = calculate_similarity("ELDEN RING", "elden ring");
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_duplicate_detection_with_metadata_noise() {
        let results = vec![
            make_result("fitgirl", "Elden Ring [45 GB] v1.0"),
            make_result("dodi", "Elden Ring [50 GB] v1.1"),
        ];

        let duplicates = find_duplicates_with_threshold(&results, 0.85);
        assert_eq!(duplicates.len(), 1);
    }

    #[test]
    fn test_threshold_clamping() {
        let analyzer = ContentAnalyzer::with_threshold(1.5);
        assert!((analyzer.duplicate_threshold - 1.0).abs() < 0.01);

        let analyzer2 = ContentAnalyzer::with_threshold(-0.5);
        assert!((analyzer2.duplicate_threshold - 0.0).abs() < 0.01);
    }
}
