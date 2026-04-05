// Copyright 2026 Trame Contributors
// SPDX-License-Identifier: Apache-2.0

//! CI/CD integration configuration for DST simulation tiers.
//!
//! Defines four tiers of deterministic simulation testing, each with
//! increasing depth. CI tiers control tick count, fault intensity,
//! coverage targets, and wall-clock timeout.
//!
//! # Tiers
//!
//! | Tier         | Ticks  | Timeout  | Trigger        | Coverage Target |
//! |--------------|--------|----------|----------------|-----------------|
//! | Commit       | 10K    | 2 min    | Every push/PR  | Informational   |
//! | Nightly      | 1M     | 20 min   | Once per night | 80%             |
//! | Weekly       | 10M    | 3 hours  | Once per week  | 100%            |
//! | `PreRelease` | 100M   | 24 hours | Before release | 100% (hard)     |

use std::fmt;
use std::time::Duration;

use crate::fault::FaultProfile;

// ===========================================================================
// CiTier
// ===========================================================================

/// CI/CD testing tier with increasing simulation depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CiTier {
    /// Commit: 10K ticks, ~2 minutes. Runs on every push/PR.
    Commit,
    /// Nightly: 1M ticks, ~20 minutes. Runs once per night.
    Nightly,
    /// Weekly: 10M ticks, ~3 hours. Runs once per week.
    Weekly,
    /// Pre-release: 100M ticks, ~24 hours. Runs before each release.
    PreRelease,
}

impl CiTier {
    /// Returns all tiers in order of increasing depth.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[Self::Commit, Self::Nightly, Self::Weekly, Self::PreRelease]
    }
}

impl fmt::Display for CiTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Commit => write!(f, "commit"),
            Self::Nightly => write!(f, "nightly"),
            Self::Weekly => write!(f, "weekly"),
            Self::PreRelease => write!(f, "pre-release"),
        }
    }
}

// ===========================================================================
// TierConfig
// ===========================================================================

/// Configuration for a single CI tier.
#[derive(Debug, Clone)]
pub struct TierConfig {
    /// CI tier.
    pub tier: CiTier,
    /// Number of simulation ticks.
    pub tick_count: u64,
    /// Wall-clock timeout.
    pub timeout: Duration,
    /// Fault injection profile.
    pub fault_profile: FaultProfile,
    /// Coverage target percentage (0 = informational only).
    pub coverage_target_percent: f64,
    /// Whether to hard-fail on coverage miss.
    pub coverage_hard_gate: bool,
    /// Number of parallel seed runs.
    pub parallel_seeds: u32,
    /// Whether to run regression corpus seeds.
    pub run_regression_corpus: bool,
}

impl TierConfig {
    /// Configuration for the commit tier.
    #[must_use]
    pub fn commit() -> Self {
        Self {
            tier: CiTier::Commit,
            tick_count: 10_000,
            timeout: Duration::from_secs(120),
            fault_profile: FaultProfile::Gentle,
            coverage_target_percent: 0.0,
            coverage_hard_gate: false,
            parallel_seeds: 1,
            run_regression_corpus: true,
        }
    }

    /// Configuration for the nightly tier.
    #[must_use]
    pub fn nightly() -> Self {
        Self {
            tier: CiTier::Nightly,
            tick_count: 1_000_000,
            timeout: Duration::from_secs(1200),
            fault_profile: FaultProfile::Normal,
            coverage_target_percent: 80.0,
            coverage_hard_gate: true,
            parallel_seeds: 4,
            run_regression_corpus: true,
        }
    }

    /// Configuration for the weekly tier.
    #[must_use]
    pub fn weekly() -> Self {
        Self {
            tier: CiTier::Weekly,
            tick_count: 10_000_000,
            timeout: Duration::from_secs(10_800),
            fault_profile: FaultProfile::Aggressive,
            coverage_target_percent: 100.0,
            coverage_hard_gate: true,
            parallel_seeds: 8,
            run_regression_corpus: true,
        }
    }

    /// Configuration for the pre-release tier.
    #[must_use]
    pub fn pre_release() -> Self {
        Self {
            tier: CiTier::PreRelease,
            tick_count: 100_000_000,
            timeout: Duration::from_secs(86_400),
            fault_profile: FaultProfile::Aggressive,
            coverage_target_percent: 100.0,
            coverage_hard_gate: true,
            parallel_seeds: 16,
            run_regression_corpus: true,
        }
    }

    /// Get tier configuration by tier type.
    #[must_use]
    pub fn for_tier(tier: CiTier) -> Self {
        match tier {
            CiTier::Commit => Self::commit(),
            CiTier::Nightly => Self::nightly(),
            CiTier::Weekly => Self::weekly(),
            CiTier::PreRelease => Self::pre_release(),
        }
    }
}

impl fmt::Display for TierConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TierConfig({}: ticks={}, timeout={}s, fault_profile={:?}, \
             coverage_target={:.0}%, parallel_seeds={})",
            self.tier,
            self.tick_count,
            self.timeout.as_secs(),
            self.fault_profile,
            self.coverage_target_percent,
            self.parallel_seeds,
        )
    }
}

// ===========================================================================
// SeedRecord
// ===========================================================================

/// A seed record for the regression corpus.
///
/// Failing seeds are stored in the regression corpus and replayed during
/// future CI runs to prevent regressions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SeedRecord {
    /// The PRNG seed.
    pub seed: u64,
    /// Property violated (e.g., "P1", "INV-S1").
    pub violated_property: String,
    /// Tick at which the violation occurred.
    pub failure_tick: u64,
    /// CI tier at which the failure was discovered.
    pub discovered_tier: CiTier,
    /// Human-readable description.
    pub description: String,
}

impl fmt::Display for SeedRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SeedRecord(seed={}, property={}, tick={}, tier={})",
            self.seed, self.violated_property, self.failure_tick, self.discovered_tier,
        )
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_tiers() {
        assert_eq!(CiTier::all().len(), 4);
    }

    #[test]
    fn tier_ordering() {
        assert!(CiTier::Commit < CiTier::Nightly);
        assert!(CiTier::Nightly < CiTier::Weekly);
        assert!(CiTier::Weekly < CiTier::PreRelease);
    }

    #[test]
    fn tier_display() {
        assert_eq!(format!("{}", CiTier::Commit), "commit");
        assert_eq!(format!("{}", CiTier::Nightly), "nightly");
        assert_eq!(format!("{}", CiTier::Weekly), "weekly");
        assert_eq!(format!("{}", CiTier::PreRelease), "pre-release");
    }

    #[test]
    fn commit_config() {
        let config = TierConfig::commit();
        assert_eq!(config.tier, CiTier::Commit);
        assert_eq!(config.tick_count, 10_000);
        assert_eq!(config.timeout, Duration::from_secs(120));
        assert!(!config.coverage_hard_gate);
    }

    #[test]
    fn nightly_config() {
        let config = TierConfig::nightly();
        assert_eq!(config.tier, CiTier::Nightly);
        assert_eq!(config.tick_count, 1_000_000);
        assert!(config.coverage_hard_gate);
        assert_eq!(config.parallel_seeds, 4);
    }

    #[test]
    fn weekly_config() {
        let config = TierConfig::weekly();
        assert_eq!(config.tier, CiTier::Weekly);
        assert_eq!(config.tick_count, 10_000_000);
        assert!(config.coverage_hard_gate);
    }

    #[test]
    fn pre_release_config() {
        let config = TierConfig::pre_release();
        assert_eq!(config.tier, CiTier::PreRelease);
        assert_eq!(config.tick_count, 100_000_000);
        assert_eq!(config.timeout, Duration::from_secs(86_400));
    }

    #[test]
    fn for_tier_dispatch() {
        for &tier in CiTier::all() {
            let config = TierConfig::for_tier(tier);
            assert_eq!(config.tier, tier);
        }
    }

    #[test]
    fn increasing_tick_counts() {
        let ticks: Vec<u64> = CiTier::all()
            .iter()
            .map(|&t| TierConfig::for_tier(t).tick_count)
            .collect();
        for window in ticks.windows(2) {
            assert!(
                window[1] > window[0],
                "Tier tick counts should increase: {} <= {}",
                window[1],
                window[0]
            );
        }
    }

    #[test]
    fn increasing_parallel_seeds() {
        let seeds: Vec<u32> = CiTier::all()
            .iter()
            .map(|&t| TierConfig::for_tier(t).parallel_seeds)
            .collect();
        for window in seeds.windows(2) {
            assert!(
                window[1] >= window[0],
                "Parallel seeds should not decrease: {} < {}",
                window[1],
                window[0]
            );
        }
    }

    #[test]
    fn tier_config_display() {
        let config = TierConfig::commit();
        let display = format!("{config}");
        assert!(display.contains("commit"));
        assert!(display.contains("10000"));
    }

    #[test]
    fn seed_record_display() {
        let record = SeedRecord {
            seed: 42,
            violated_property: "P1".to_string(),
            failure_tick: 1000,
            discovered_tier: CiTier::Nightly,
            description: "test failure".to_string(),
        };
        let display = format!("{record}");
        assert!(display.contains("42"));
        assert!(display.contains("P1"));
        assert!(display.contains("nightly"));
    }
}
