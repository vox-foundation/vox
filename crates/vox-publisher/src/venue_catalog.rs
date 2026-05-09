//! Venue catalog: whitelisted publication venues + journal-fit recommender.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VenueType {
    Conference,
    Journal,
    WebNative,
    LivingReview,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewModel {
    DoubleBlind,
    Open,
    Editorial,
    Internal,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VenueEntry {
    pub id: String,
    pub name: String,
    pub shortname: String,
    #[serde(rename = "type")]
    pub venue_type: VenueType,
    pub primary: bool,
    pub focus: Vec<String>,
    pub review_model: ReviewModel,
    pub typical_deadline: Option<String>,
    pub min_pages: Option<u32>,
    pub max_pages: Option<u32>,
    pub target_atlas: Option<String>,
    #[serde(default)]
    pub living_review: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VenueCatalog {
    pub schema_version: u32,
    pub venues: Vec<VenueEntry>,
}

impl VenueCatalog {
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    pub fn primary_venues(&self) -> impl Iterator<Item = &VenueEntry> {
        self.venues.iter().filter(|v| v.primary)
    }

    pub fn find_by_id(&self, id: &str) -> Option<&VenueEntry> {
        self.venues.iter().find(|v| v.id == id)
    }

    pub fn whitelisted_venue_ids(&self) -> Vec<String> {
        self.venues.iter().map(|v| v.id.clone()).collect()
    }
}

/// Score of a single venue for a given set of finding topics.
#[derive(Debug, Clone, Serialize)]
pub struct VenueFitScore {
    pub venue_id: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// Rank venues by topic overlap with a finding's focus areas.
pub struct JournalFitRecommender<'a> {
    catalog: &'a VenueCatalog,
}

impl<'a> JournalFitRecommender<'a> {
    pub fn new(catalog: &'a VenueCatalog) -> Self {
        Self { catalog }
    }

    /// Score each venue by overlap between `finding_topics` and `venue.focus`.
    /// Returns venues sorted descending by score (highest fit first).
    pub fn rank_venues(&self, finding_topics: &[&str]) -> Vec<VenueFitScore> {
        let mut scores: Vec<VenueFitScore> = self
            .catalog
            .venues
            .iter()
            .map(|v| {
                let matches: Vec<String> = finding_topics
                    .iter()
                    .filter(|t| v.focus.iter().any(|f| f == *t))
                    .map(|t| t.to_string())
                    .collect();
                let score = if v.focus.is_empty() {
                    0.0
                } else {
                    matches.len() as f64 / v.focus.len() as f64
                };
                VenueFitScore {
                    venue_id: v.id.clone(),
                    score,
                    reasons: matches,
                }
            })
            .collect();
        scores.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scores
    }

    /// Return the best-fit venue entry, or None if no venues score > 0.
    pub fn top_venue(&self, finding_topics: &[&str]) -> Option<&'a VenueEntry> {
        let scores = self.rank_venues(finding_topics);
        scores
            .into_iter()
            .find(|s| s.score > 0.0)
            .and_then(|s| self.catalog.find_by_id(&s.venue_id))
    }

    /// True iff `venue_id` appears in the catalog whitelist.
    pub fn is_venue_whitelisted(&self, venue_id: &str) -> bool {
        self.catalog.find_by_id(venue_id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YAML: &str = r#"
schema_version: 1
venues:
  - id: imc
    name: "Internet Measurement Conference"
    shortname: "IMC"
    type: conference
    primary: true
    focus: ["measurement", "provider-behavior"]
    review_model: double-blind
    typical_deadline: "June"
    min_pages: 12
    max_pages: 14
    target_atlas: provider-atlas
  - id: tmlr
    name: "Transactions on Machine Learning Research"
    shortname: "TMLR"
    type: journal
    primary: true
    focus: ["machine-learning", "empirical"]
    review_model: open
    typical_deadline: "rolling"
    min_pages: ~
    max_pages: ~
    target_atlas: ~
  - id: distill
    name: "Distill"
    shortname: "Distill"
    type: web-native
    primary: false
    focus: ["explanation"]
    review_model: editorial
    typical_deadline: ~
    min_pages: ~
    max_pages: ~
    target_atlas: ~
"#;

    #[test]
    fn parse_venue_catalog() {
        let cat = VenueCatalog::from_yaml(SAMPLE_YAML).expect("parse");
        assert_eq!(cat.schema_version, 1);
        assert_eq!(cat.venues.len(), 3);
        let imc = cat.find_by_id("imc").expect("imc");
        assert_eq!(imc.shortname, "IMC");
        assert_eq!(imc.venue_type, VenueType::Conference);
        assert!(imc.primary);
        assert_eq!(imc.min_pages, Some(12));
    }

    #[test]
    fn primary_venues_filter() {
        let cat = VenueCatalog::from_yaml(SAMPLE_YAML).expect("parse");
        let primary: Vec<_> = cat.primary_venues().collect();
        assert_eq!(primary.len(), 2);
    }

    #[test]
    fn journal_fit_recommender_ranks_by_topic_overlap() {
        let cat = VenueCatalog::from_yaml(SAMPLE_YAML).expect("parse");
        let rec = JournalFitRecommender::new(&cat);
        let scores = rec.rank_venues(&["provider-behavior", "measurement"]);
        assert!(!scores.is_empty());
        assert_eq!(scores[0].venue_id, "imc");
        assert!(scores[0].score > 0.0);
    }

    #[test]
    fn whitelist_check() {
        let cat = VenueCatalog::from_yaml(SAMPLE_YAML).expect("parse");
        let rec = JournalFitRecommender::new(&cat);
        assert!(rec.is_venue_whitelisted("imc"));
        assert!(rec.is_venue_whitelisted("tmlr"));
        assert!(!rec.is_venue_whitelisted("predatory-journal-xyz"));
    }
}
