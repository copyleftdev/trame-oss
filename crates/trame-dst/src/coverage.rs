// Copyright 2026 Trame Contributors
// SPDX-License-Identifier: Apache-2.0

//! Variant coverage tracking for DST simulation.
//!
//! Tracks which operation types and fault types have been exercised during
//! simulation. Coverage tracking enables:
//!
//! - Identification of untested scenarios (gap analysis)
//! - PRNG scheduler weight adjustments for under-covered paths
//! - CI coverage gates (informational at commit, hard gate at release)

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::fault::FaultType;

// ===========================================================================
// VariantId
// ===========================================================================

/// Unique identifier for a test variant.
///
/// A variant represents a code path, operation type, fault combination,
/// state transition, or property rule that must be tested.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariantId(pub String);

impl VariantId {
    /// Create a new variant ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VariantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for VariantId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for VariantId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

// ===========================================================================
// CoverageGap
// ===========================================================================

/// A gap in coverage: a variant that has not been exercised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageGap {
    /// The unexercised variant.
    pub variant_id: VariantId,
    /// Category of the variant (for grouping in reports).
    pub category: String,
}

// ===========================================================================
// CoverageReport
// ===========================================================================

/// Summary report of variant coverage.
#[derive(Debug, Clone)]
pub struct CoverageReport {
    /// Total registered variants.
    pub total_variants: usize,
    /// Variants exercised at least once.
    pub covered_variants: usize,
    /// Variants not yet exercised.
    pub gaps: Vec<CoverageGap>,
    /// Coverage percentage (0.0 - 100.0).
    pub coverage_percent: f64,
    /// Per-category coverage percentages.
    pub category_coverage: BTreeMap<String, f64>,
}

impl fmt::Display for CoverageReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Coverage: {}/{} ({:.1}%), {} gaps",
            self.covered_variants,
            self.total_variants,
            self.coverage_percent,
            self.gaps.len(),
        )
    }
}

// ===========================================================================
// WeightAdjustment
// ===========================================================================

/// Weight adjustment for a variant to increase its probability.
#[derive(Debug, Clone)]
pub struct WeightAdjustment {
    /// The variant being adjusted.
    pub variant_id: VariantId,
    /// Recommended weight multiplier (> 1.0 means increase probability).
    pub multiplier: f64,
}

// ===========================================================================
// CoverageTracker
// ===========================================================================

/// Tracks which variants have been exercised during simulation.
///
/// Register all expected variants at startup, then mark them as covered
/// as they are exercised. Query for gaps and weight adjustments.
pub struct CoverageTracker {
    /// All registered variants with their category.
    registered: BTreeMap<VariantId, String>,
    /// Set of variant IDs that have been exercised.
    covered: BTreeSet<VariantId>,
    /// Exercised fault types.
    exercised_faults: BTreeSet<FaultType>,
    /// Exercised operation types (by string ID).
    exercised_operations: BTreeSet<String>,
    /// Hit counts per variant.
    hit_counts: BTreeMap<VariantId, u64>,
}

impl CoverageTracker {
    /// Create a new empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registered: BTreeMap::new(),
            covered: BTreeSet::new(),
            exercised_faults: BTreeSet::new(),
            exercised_operations: BTreeSet::new(),
            hit_counts: BTreeMap::new(),
        }
    }

    /// Register a variant that should be covered.
    pub fn register(&mut self, id: impl Into<VariantId>, category: impl Into<String>) {
        let id = id.into();
        self.registered.insert(id, category.into());
    }

    /// Mark a variant as covered.
    pub fn mark_covered(&mut self, id: &VariantId) {
        self.covered.insert(id.clone());
        *self.hit_counts.entry(id.clone()).or_insert(0) += 1;
    }

    /// Record that a fault type was exercised.
    pub fn record_fault(&mut self, fault_type: FaultType) {
        self.exercised_faults.insert(fault_type);
        let variant_id = VariantId::new(format!("fault:{}", fault_type.spec_id()));
        self.mark_covered(&variant_id);
    }

    /// Record that an operation type was exercised.
    pub fn record_operation(&mut self, op_type: &str) {
        self.exercised_operations.insert(op_type.to_string());
        let variant_id = VariantId::new(format!("op:{op_type}"));
        self.mark_covered(&variant_id);
    }

    /// Total registered variants.
    #[must_use]
    pub fn total_registered(&self) -> usize {
        self.registered.len()
    }

    /// Number of covered variants.
    #[must_use]
    pub fn total_covered(&self) -> usize {
        self.registered
            .keys()
            .filter(|id| self.covered.contains(*id))
            .count()
    }

    /// Coverage percentage (0.0 - 100.0).
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn coverage_percent(&self) -> f64 {
        if self.registered.is_empty() {
            return 100.0;
        }
        self.total_covered() as f64 / self.registered.len() as f64 * 100.0
    }

    /// Generate a coverage report.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn coverage_report(&self) -> CoverageReport {
        let mut gaps = Vec::new();
        let mut category_total: BTreeMap<String, usize> = BTreeMap::new();
        let mut category_covered: BTreeMap<String, usize> = BTreeMap::new();

        for (id, category) in &self.registered {
            *category_total.entry(category.clone()).or_insert(0) += 1;
            if self.covered.contains(id) {
                *category_covered.entry(category.clone()).or_insert(0) += 1;
            } else {
                gaps.push(CoverageGap {
                    variant_id: id.clone(),
                    category: category.clone(),
                });
            }
        }

        let category_coverage: BTreeMap<String, f64> = category_total
            .iter()
            .map(|(cat, &total)| {
                let covered = category_covered.get(cat).copied().unwrap_or(0);
                let pct = if total == 0 {
                    100.0
                } else {
                    covered as f64 / total as f64 * 100.0
                };
                (cat.clone(), pct)
            })
            .collect();

        CoverageReport {
            total_variants: self.registered.len(),
            covered_variants: self.total_covered(),
            gaps,
            coverage_percent: self.coverage_percent(),
            category_coverage,
        }
    }

    /// Generate weight adjustments for under-covered variants.
    ///
    /// Returns a list of adjustments where variants with fewer hits
    /// get higher multipliers to increase their probability.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn weight_adjustments(&self) -> Vec<WeightAdjustment> {
        if self.registered.is_empty() {
            return Vec::new();
        }

        let max_hits = self
            .hit_counts
            .values()
            .copied()
            .max()
            .unwrap_or(1)
            .max(1);

        let mut adjustments = Vec::new();

        for id in self.registered.keys() {
            let hits = self.hit_counts.get(id).copied().unwrap_or(0);
            // Inverse proportion: less hits = higher multiplier
            let multiplier = if hits == 0 {
                // Never hit: max boost
                (max_hits as f64) + 1.0
            } else {
                max_hits as f64 / hits as f64
            };

            if multiplier > 1.0 {
                adjustments.push(WeightAdjustment {
                    variant_id: id.clone(),
                    multiplier,
                });
            }
        }

        adjustments
    }

    /// Returns the set of exercised fault types.
    #[must_use]
    pub fn exercised_faults(&self) -> &BTreeSet<FaultType> {
        &self.exercised_faults
    }

    /// Returns the set of exercised operation types.
    #[must_use]
    pub fn exercised_operations(&self) -> &BTreeSet<String> {
        &self.exercised_operations
    }

    /// Hit count for a specific variant.
    #[must_use]
    pub fn hit_count(&self, id: &VariantId) -> u64 {
        self.hit_counts.get(id).copied().unwrap_or(0)
    }
}

impl Default for CoverageTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tracker_100_percent() {
        let tracker = CoverageTracker::new();
        assert!((tracker.coverage_percent() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn register_and_cover() {
        let mut tracker = CoverageTracker::new();
        tracker.register("V-001", "storage");
        tracker.register("V-002", "storage");
        tracker.register("V-003", "network");

        assert_eq!(tracker.total_registered(), 3);
        assert_eq!(tracker.total_covered(), 0);

        tracker.mark_covered(&VariantId::new("V-001"));
        assert_eq!(tracker.total_covered(), 1);

        tracker.mark_covered(&VariantId::new("V-002"));
        tracker.mark_covered(&VariantId::new("V-003"));
        assert_eq!(tracker.total_covered(), 3);
        assert!((tracker.coverage_percent() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn coverage_report_gaps() {
        let mut tracker = CoverageTracker::new();
        tracker.register("V-001", "storage");
        tracker.register("V-002", "storage");
        tracker.register("V-003", "network");

        tracker.mark_covered(&VariantId::new("V-001"));

        let report = tracker.coverage_report();
        assert_eq!(report.total_variants, 3);
        assert_eq!(report.covered_variants, 1);
        assert_eq!(report.gaps.len(), 2);
        assert!(report.coverage_percent < 50.0);
    }

    #[test]
    fn coverage_report_category_breakdown() {
        let mut tracker = CoverageTracker::new();
        tracker.register("S-001", "storage");
        tracker.register("S-002", "storage");
        tracker.register("N-001", "network");

        tracker.mark_covered(&VariantId::new("S-001"));

        let report = tracker.coverage_report();
        assert_eq!(report.category_coverage.get("storage"), Some(&50.0));
        assert_eq!(report.category_coverage.get("network"), Some(&0.0));
    }

    #[test]
    fn record_fault_type() {
        let mut tracker = CoverageTracker::new();
        let variant_id = VariantId::new("fault:S-001");
        tracker.register(variant_id.clone(), "fault");

        tracker.record_fault(FaultType::StorageWriteFailure);
        assert!(tracker.exercised_faults().contains(&FaultType::StorageWriteFailure));
        assert_eq!(tracker.hit_count(&variant_id), 1);
    }

    #[test]
    fn record_operation_type() {
        let mut tracker = CoverageTracker::new();
        let variant_id = VariantId::new("op:write");
        tracker.register(variant_id.clone(), "operation");

        tracker.record_operation("write");
        assert!(tracker.exercised_operations().contains("write"));
        assert_eq!(tracker.hit_count(&variant_id), 1);
    }

    #[test]
    fn weight_adjustments_boost_uncovered() {
        let mut tracker = CoverageTracker::new();
        tracker.register("V-001", "a");
        tracker.register("V-002", "a");
        tracker.register("V-003", "a");

        // Hit V-001 many times, V-002 once, V-003 never
        for _ in 0..10 {
            tracker.mark_covered(&VariantId::new("V-001"));
        }
        tracker.mark_covered(&VariantId::new("V-002"));

        let adjustments = tracker.weight_adjustments();
        assert!(!adjustments.is_empty());

        // V-003 (never hit) should have the highest multiplier
        let v003_adj = adjustments
            .iter()
            .find(|a| a.variant_id.as_str() == "V-003");
        assert!(v003_adj.is_some());
        assert!(v003_adj.unwrap().multiplier > 1.0);
    }

    #[test]
    fn coverage_report_display() {
        let mut tracker = CoverageTracker::new();
        tracker.register("V-001", "a");
        tracker.mark_covered(&VariantId::new("V-001"));

        let report = tracker.coverage_report();
        let display = format!("{report}");
        assert!(display.contains("100.0%"));
        assert!(display.contains("0 gaps"));
    }

    #[test]
    fn variant_id_from_str() {
        let id: VariantId = "test".into();
        assert_eq!(id.as_str(), "test");
    }

    #[test]
    fn variant_id_display() {
        let id = VariantId::new("V-001");
        assert_eq!(format!("{id}"), "V-001");
    }

    #[test]
    fn hit_count_increments() {
        let mut tracker = CoverageTracker::new();
        tracker.register("V-001", "a");

        let id = VariantId::new("V-001");
        assert_eq!(tracker.hit_count(&id), 0);

        tracker.mark_covered(&id);
        assert_eq!(tracker.hit_count(&id), 1);

        tracker.mark_covered(&id);
        assert_eq!(tracker.hit_count(&id), 2);
    }

    #[test]
    fn all_fault_types_registerable() {
        let mut tracker = CoverageTracker::new();
        for ft in FaultType::all() {
            let id = VariantId::new(format!("fault:{}", ft.spec_id()));
            tracker.register(id, "fault");
        }
        assert_eq!(tracker.total_registered(), 32);
    }
}
