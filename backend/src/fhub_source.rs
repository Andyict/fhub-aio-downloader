//! FHub source boundary
//!
//! Native boundary types for source discovery, ingest planning, and activity creation.
//! This module is intentionally small and additive so larger search/orchestrator flows
//! can migrate behind FHUB-owned types without breaking current API contracts.

use serde::{Deserialize, Serialize};

/// A normalized source candidate that FHUB can score, display, or convert into an activity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FhubSourceCandidate {
    pub code: String,
    pub title: String,
    pub url: String,
    pub size: u64,
    pub quality: Option<String>,
    pub resolution: Option<String>,
    pub source: Option<String>,
}

impl FhubSourceCandidate {
    pub fn new(code: impl Into<String>, title: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            title: title.into(),
            url: url.into(),
            size: 0,
            quality: None,
            resolution: None,
            source: None,
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn with_quality(
        mut self,
        quality: Option<String>,
        resolution: Option<String>,
        source: Option<String>,
    ) -> Self {
        self.quality = quality;
        self.resolution = resolution;
        self.source = source;
        self
    }

    pub fn normalized_code(&self) -> String {
        self.code.split('?').next().unwrap_or(&self.code).trim().to_string()
    }
}

/// A grouped ingest plan used by FHUB-native workflows before tasks are created.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FhubIngestPlan {
    pub title: String,
    pub media_type: String,
    pub candidates: Vec<FhubSourceCandidate>,
}

impl FhubIngestPlan {
    pub fn new(title: impl Into<String>, media_type: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            media_type: media_type.into(),
            candidates: Vec::new(),
        }
    }

    pub fn push_candidate(&mut self, candidate: FhubSourceCandidate) {
        self.candidates.push(candidate);
    }

    pub fn total_size(&self) -> u64 {
        self.candidates.iter().map(|candidate| candidate.size).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Remove duplicate candidates by normalized source code while preserving first-seen order.
    pub fn deduplicate_by_code(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.candidates.retain(|candidate| seen.insert(candidate.normalized_code()));
    }

    /// Sort largest files first. This gives FHUB a deterministic candidate order for review screens.
    pub fn sort_by_size_desc(&mut self) {
        self.candidates.sort_by(|a, b| b.size.cmp(&a.size));
    }

    /// Apply FHUB-native candidate normalization in a deterministic order.
    pub fn normalize(&mut self) {
        self.deduplicate_by_code();
        self.sort_by_size_desc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_builder_sets_core_fields() {
        let candidate = FhubSourceCandidate::new("abc", "Movie", "https://example.test/file")
            .with_size(42)
            .with_quality(Some("WEB-DL".to_string()), Some("1080p".to_string()), None);

        assert_eq!(candidate.code, "abc");
        assert_eq!(candidate.title, "Movie");
        assert_eq!(candidate.size, 42);
        assert_eq!(candidate.quality.as_deref(), Some("WEB-DL"));
        assert_eq!(candidate.resolution.as_deref(), Some("1080p"));
    }

    #[test]
    fn candidate_normalizes_code_without_query_string() {
        let candidate = FhubSourceCandidate::new("abc?token=123", "Movie", "https://example.test/file");
        assert_eq!(candidate.normalized_code(), "abc");
    }

    #[test]
    fn ingest_plan_tracks_candidates_and_size() {
        let mut plan = FhubIngestPlan::new("Series", "tv");
        assert!(plan.is_empty());

        plan.push_candidate(FhubSourceCandidate::new("one", "Episode 1", "https://example.test/1").with_size(10));
        plan.push_candidate(FhubSourceCandidate::new("two", "Episode 2", "https://example.test/2").with_size(15));

        assert!(!plan.is_empty());
        assert_eq!(plan.total_size(), 25);
    }

    #[test]
    fn ingest_plan_deduplicates_by_normalized_code() {
        let mut plan = FhubIngestPlan::new("Movie", "movie");
        plan.push_candidate(FhubSourceCandidate::new("abc?token=1", "A", "https://example.test/a").with_size(10));
        plan.push_candidate(FhubSourceCandidate::new("abc?token=2", "A Copy", "https://example.test/a-copy").with_size(20));
        plan.push_candidate(FhubSourceCandidate::new("def", "B", "https://example.test/b").with_size(30));

        plan.deduplicate_by_code();

        assert_eq!(plan.candidates.len(), 2);
        assert_eq!(plan.candidates[0].title, "A");
        assert_eq!(plan.candidates[1].code, "def");
    }

    #[test]
    fn ingest_plan_normalizes_to_unique_largest_first() {
        let mut plan = FhubIngestPlan::new("Movie", "movie");
        plan.push_candidate(FhubSourceCandidate::new("small", "Small", "https://example.test/small").with_size(10));
        plan.push_candidate(FhubSourceCandidate::new("large", "Large", "https://example.test/large").with_size(100));
        plan.push_candidate(FhubSourceCandidate::new("small?dup=1", "Small copy", "https://example.test/small-copy").with_size(20));

        plan.normalize();

        assert_eq!(plan.candidates.len(), 2);
        assert_eq!(plan.candidates[0].code, "large");
        assert_eq!(plan.candidates[1].code, "small");
    }
}
