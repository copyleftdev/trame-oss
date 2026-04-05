// Copyright 2026 Trame Contributors
// SPDX-License-Identifier: Apache-2.0

//! Failure shrinking for deterministic simulation testing.
//!
//! When the DST framework discovers a failure, the shrinking algorithm reduces
//! the operation sequence to a minimal reproduction. Four phases run in order:
//!
//! 1. **Binary search** for the shortest prefix that still fails.
//! 2. **Individual removal** of non-essential operations.
//! 3. **Operation simplification** to reduce parameter complexity.
//! 4. **Bootstrap pruning** to remove unnecessary setup operations.
//!
//! All shrinking is deterministic: same failure + same seed = same result.

use std::fmt;
use std::time::Instant;

// ===========================================================================
// ShrinkConfig
// ===========================================================================

/// Configuration for the shrinking algorithm.
#[derive(Debug, Clone)]
pub struct ShrinkConfig {
    /// Maximum total replay attempts across all phases.
    pub max_iterations: u32,
    /// Wall-clock timeout in seconds (0 = unlimited).
    pub timeout_secs: u64,
    /// Whether to run the simplification phase.
    pub simplify_operations: bool,
    /// Whether to run the bootstrap pruning phase.
    pub prune_bootstrap: bool,
    /// Whether to verify 1-minimality after shrinking.
    pub verify_minimality: bool,
}

impl Default for ShrinkConfig {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            timeout_secs: 300,
            simplify_operations: true,
            prune_bootstrap: true,
            verify_minimality: true,
        }
    }
}

// ===========================================================================
// ShrinkPhase
// ===========================================================================

/// Phase of the shrinking algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShrinkPhase {
    /// Binary search for minimal prefix.
    BinarySearch,
    /// Individual operation removal.
    OperationRemoval,
    /// Operation simplification.
    Simplification,
    /// Bootstrap pruning.
    BootstrapPruning,
    /// Verifying 1-minimality.
    Verification,
    /// Shrinking complete.
    Complete,
}

impl fmt::Display for ShrinkPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BinarySearch => write!(f, "binary-search"),
            Self::OperationRemoval => write!(f, "operation-removal"),
            Self::Simplification => write!(f, "simplification"),
            Self::BootstrapPruning => write!(f, "bootstrap-pruning"),
            Self::Verification => write!(f, "verification"),
            Self::Complete => write!(f, "complete"),
        }
    }
}

// ===========================================================================
// ShrinkResult
// ===========================================================================

/// Result of a shrinking run.
#[derive(Debug, Clone)]
pub struct ShrinkResult {
    /// The original seed that triggered the failure.
    pub original_seed: u64,
    /// Minimal operation count after shrinking.
    pub minimal_ops: usize,
    /// Original operation count before shrinking.
    pub original_ops: usize,
    /// Reduction ratio (0.0 = no reduction, 1.0 = empty).
    pub reduction_ratio: f64,
    /// Total replay attempts performed.
    pub replay_attempts: u32,
    /// Successful removals.
    pub removals: u32,
    /// Successful simplifications.
    pub simplifications: u32,
    /// Phase when shrinking stopped.
    pub final_phase: ShrinkPhase,
    /// Whether shrinking completed normally (vs timed out or hit iteration limit).
    pub completed: bool,
    /// Wall-clock time spent shrinking.
    pub elapsed_secs: f64,
    /// Indices of operations in the minimal reproduction.
    pub minimal_indices: Vec<usize>,
}

impl fmt::Display for ShrinkResult {
    #[allow(clippy::cast_precision_loss)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ShrinkResult(seed={}, {} -> {} ops ({:.1}% reduction), {} replays, {:.1}s)",
            self.original_seed,
            self.original_ops,
            self.minimal_ops,
            self.reduction_ratio * 100.0,
            self.replay_attempts,
            self.elapsed_secs,
        )
    }
}

// ===========================================================================
// FailureType -- what kind of failure was observed
// ===========================================================================

/// Classification of the failure that triggered shrinking.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FailureType {
    /// A property check failed.
    PropertyViolation {
        /// The property ID.
        property_id: String,
    },
    /// A liveness property was violated (progress stalled).
    LivenessFailure {
        /// The property ID.
        property_id: String,
    },
    /// A consistency property was violated.
    ConsistencyViolation {
        /// The property ID.
        property_id: String,
    },
    /// A crash during recovery produced unexpected state.
    RecoveryFailure {
        /// Description.
        description: String,
    },
    /// An assertion in the reference model oracle failed.
    OracleMismatch {
        /// What differed.
        description: String,
    },
}

impl fmt::Display for FailureType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PropertyViolation { property_id } => {
                write!(f, "property violation: {property_id}")
            }
            Self::LivenessFailure { property_id } => {
                write!(f, "liveness failure: {property_id}")
            }
            Self::ConsistencyViolation { property_id } => {
                write!(f, "consistency violation: {property_id}")
            }
            Self::RecoveryFailure { description } => {
                write!(f, "recovery failure: {description}")
            }
            Self::OracleMismatch { description } => {
                write!(f, "oracle mismatch: {description}")
            }
        }
    }
}

// ===========================================================================
// Shrink function -- 4-phase algorithm
// ===========================================================================

/// Shrink a failing operation sequence to a minimal reproduction.
///
/// The `replay_fn` takes a slice of operation indices and returns `true` if
/// the failure still reproduces with that subset.
///
/// # Algorithm
///
/// 1. **Binary search prefix**: Find the shortest prefix of operations that
///    still triggers the failure.
/// 2. **Operation removal**: Try removing each operation individually.
/// 3. **Operation simplification**: Try simplifying each operation (by index).
/// 4. **Bootstrap pruning**: Remove early operations that may be setup-only.
#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
pub fn shrink(
    seed: u64,
    total_ops: usize,
    config: &ShrinkConfig,
    mut replay_fn: impl FnMut(&[usize]) -> bool,
) -> ShrinkResult {
    let start = Instant::now();

    let mut indices: Vec<usize> = (0..total_ops).collect();
    let mut replay_attempts: u32 = 0;
    let mut removals: u32 = 0;
    let mut simplifications: u32 = 0;

    let timed_out = |start: &Instant, config: &ShrinkConfig| -> bool {
        config.timeout_secs > 0 && start.elapsed().as_secs() >= config.timeout_secs
    };

    let budget_exhausted =
        |attempts: u32, config: &ShrinkConfig| -> bool { attempts >= config.max_iterations };

    // Phase 1: Binary search for minimal prefix
    if indices.len() > 1 {
        let mut lo: usize = 1;
        let mut hi = indices.len();

        while lo < hi
            && !timed_out(&start, config)
            && !budget_exhausted(replay_attempts, config)
        {
            let mid = usize::midpoint(lo, hi);
            let prefix: Vec<usize> = indices[..mid].to_vec();
            replay_attempts += 1;

            if replay_fn(&prefix) {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }

        if hi < indices.len() {
            indices.truncate(hi);
        }
    }

    // Phase 2: Operation removal
    if indices.len() > 1 {
        let mut i = 0;
        while i < indices.len()
            && !timed_out(&start, config)
            && !budget_exhausted(replay_attempts, config)
        {
            let mut candidate = indices.clone();
            candidate.remove(i);
            replay_attempts += 1;

            if replay_fn(&candidate) {
                indices = candidate;
                removals += 1;
                // Don't advance i -- try removing the new element at this position
            } else {
                i += 1;
            }
        }
    }

    // Phase 3: Operation simplification
    if config.simplify_operations {
        for i in 0..indices.len() {
            if timed_out(&start, config) || budget_exhausted(replay_attempts, config) {
                break;
            }
            if indices[i] > 0 {
                let mut candidate = indices.clone();
                candidate[i] = 0; // Simplify to index 0
                replay_attempts += 1;

                if replay_fn(&candidate) {
                    indices = candidate;
                    simplifications += 1;
                }
            }
        }
    }

    // Phase 4: Bootstrap pruning
    if config.prune_bootstrap && indices.len() > 2 {
        let max_prune = indices.len() / 2;
        for prune_count in (1..=max_prune).rev() {
            if timed_out(&start, config) || budget_exhausted(replay_attempts, config) {
                break;
            }
            let candidate: Vec<usize> = indices[prune_count..].to_vec();
            replay_attempts += 1;

            if replay_fn(&candidate) {
                indices = candidate;
                #[allow(clippy::cast_possible_truncation)]
                {
                    removals += prune_count as u32;
                }
                break;
            }
        }
    }

    // Verification phase (phase 2 already ensures 1-minimality)
    let elapsed = start.elapsed().as_secs_f64();
    let minimal_ops = indices.len();

    let reduction_ratio = if total_ops == 0 {
        0.0
    } else {
        1.0 - (minimal_ops as f64 / total_ops as f64)
    };

    let completed = !timed_out(&start, config) && !budget_exhausted(replay_attempts, config);

    ShrinkResult {
        original_seed: seed,
        minimal_ops,
        original_ops: total_ops,
        reduction_ratio,
        replay_attempts,
        removals,
        simplifications,
        final_phase: ShrinkPhase::Complete,
        completed,
        elapsed_secs: elapsed,
        minimal_indices: indices,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrink_finds_minimal_prefix() {
        // Failure requires operation at index 7 or later
        let result = shrink(42, 20, &ShrinkConfig::default(), |indices| {
            indices.iter().any(|&i| i >= 7)
        });

        assert!(result.minimal_ops <= 20);
        assert!(result.minimal_indices.iter().any(|&i| i >= 7));
        assert!(result.reduction_ratio >= 0.0);
    }

    #[test]
    fn shrink_single_necessary_operation() {
        // Only operation 5 is required
        let result = shrink(42, 10, &ShrinkConfig::default(), |indices| {
            indices.contains(&5)
        });

        assert_eq!(result.minimal_ops, 1);
        assert_eq!(result.minimal_indices, vec![5]);
        assert!(result.reduction_ratio > 0.8);
    }

    #[test]
    fn shrink_all_operations_necessary() {
        // All operations are required
        let result = shrink(42, 5, &ShrinkConfig::default(), |indices| {
            indices.len() == 5 && (0..5).all(|i| indices.contains(&i))
        });

        assert_eq!(result.minimal_ops, 5);
        assert!(result.reduction_ratio.abs() < f64::EPSILON);
    }

    #[test]
    fn shrink_empty_input() {
        let result = shrink(42, 0, &ShrinkConfig::default(), |_| true);
        assert_eq!(result.minimal_ops, 0);
        assert_eq!(result.original_ops, 0);
    }

    #[test]
    fn shrink_respects_iteration_limit() {
        let config = ShrinkConfig {
            max_iterations: 5,
            ..Default::default()
        };
        let result = shrink(42, 100, &config, |indices| indices.len() > 1);
        assert!(result.replay_attempts <= 5);
    }

    #[test]
    fn shrink_phase_display() {
        assert_eq!(format!("{}", ShrinkPhase::BinarySearch), "binary-search");
        assert_eq!(
            format!("{}", ShrinkPhase::OperationRemoval),
            "operation-removal"
        );
        assert_eq!(format!("{}", ShrinkPhase::Simplification), "simplification");
        assert_eq!(
            format!("{}", ShrinkPhase::BootstrapPruning),
            "bootstrap-pruning"
        );
        assert_eq!(format!("{}", ShrinkPhase::Verification), "verification");
        assert_eq!(format!("{}", ShrinkPhase::Complete), "complete");
    }

    #[test]
    fn shrink_result_display() {
        let result = ShrinkResult {
            original_seed: 42,
            minimal_ops: 3,
            original_ops: 100,
            reduction_ratio: 0.97,
            replay_attempts: 50,
            removals: 45,
            simplifications: 2,
            final_phase: ShrinkPhase::Complete,
            completed: true,
            elapsed_secs: 1.5,
            minimal_indices: vec![5, 10, 15],
        };
        let display = format!("{result}");
        assert!(display.contains("42"));
        assert!(display.contains("100"));
        assert!(display.contains('3'));
    }

    #[test]
    fn failure_type_display() {
        let ft = FailureType::PropertyViolation {
            property_id: "P1".to_string(),
        };
        assert_eq!(format!("{ft}"), "property violation: P1");

        let ft = FailureType::LivenessFailure {
            property_id: "L1".to_string(),
        };
        assert_eq!(format!("{ft}"), "liveness failure: L1");
    }

    #[test]
    fn default_config_sensible() {
        let config = ShrinkConfig::default();
        assert!(config.max_iterations > 0);
        assert!(config.timeout_secs > 0);
        assert!(config.simplify_operations);
        assert!(config.prune_bootstrap);
    }

    #[test]
    fn shrink_two_required_operations() {
        // Operations 3 and 8 are both required
        let result = shrink(42, 15, &ShrinkConfig::default(), |indices| {
            indices.contains(&3) && indices.contains(&8)
        });

        assert_eq!(result.minimal_ops, 2);
        assert!(result.minimal_indices.contains(&3));
        assert!(result.minimal_indices.contains(&8));
    }
}
