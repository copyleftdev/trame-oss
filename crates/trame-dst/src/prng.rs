// Copyright 2026 Trame Contributors
// SPDX-License-Identifier: Apache-2.0

//! Deterministic PRNG for simulation testing.
//!
//! Implements `SplitMix64` — a fast, statistically sound, splittable
//! generator. All randomness in the DST framework derives from a single
//! root seed via a fork hierarchy, ensuring perfect reproducibility.

use std::fmt;

// ===========================================================================
// SplitMix64 core
// ===========================================================================

/// `SplitMix64` deterministic pseudo-random number generator.
///
/// Produces high-quality 64-bit random numbers from a single `u64` seed.
/// The fork hierarchy allows independent sub-generators to be derived
/// from a parent without correlation.
///
/// Two runs with the same seed produce the identical output sequence.
/// No system entropy or OS random sources are used.
#[derive(Clone, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Create a new generator from a seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Generate the next `u64` in the sequence.
    #[must_use]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Fork a new independent PRNG stream from this generator's state.
    ///
    /// The child's seed is the parent's next output, so fork order is
    /// deterministic. Each forked stream is statistically independent.
    #[must_use]
    pub fn fork(&mut self) -> Self {
        Self::new(self.next_u64())
    }

    /// Return `true` with the given probability `[0.0, 1.0]`.
    #[must_use]
    pub fn chance(&mut self, probability: f64) -> bool {
        if probability >= 1.0 {
            return true;
        }
        if probability <= 0.0 {
            return false;
        }
        self.next_f64() < probability
    }

    /// Generate a uniform `f64` in `[0.0, 1.0)`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1_u64 << 53) as f64)
    }

    /// Generate a uniform `u64` in `[min, max]` (inclusive).
    ///
    /// # Panics
    ///
    /// Panics if `min > max`.
    #[must_use]
    pub fn range(&mut self, min: u64, max: u64) -> u64 {
        assert!(min <= max, "range: min ({min}) > max ({max})");
        if min == max {
            return min;
        }
        let span = max - min + 1;
        min + self.next_u64() % span
    }

    /// Generate a uniform `i64` in `[min, max]` (inclusive).
    ///
    /// # Panics
    ///
    /// Panics if `min > max`.
    #[must_use]
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    pub fn range_i64(&mut self, min: i64, max: i64) -> i64 {
        assert!(min <= max, "range_i64: min ({min}) > max ({max})");
        if min == max {
            return min;
        }
        let span = (max as u64).wrapping_sub(min as u64) + 1;
        (min as u64).wrapping_add(self.next_u64() % span) as i64
    }

    /// Choose a random element from a non-empty slice.
    ///
    /// # Panics
    ///
    /// Panics if the slice is empty.
    #[must_use]
    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        assert!(!items.is_empty(), "choose: empty slice");
        #[allow(clippy::cast_possible_truncation)]
        let idx = (self.next_u64() % items.len() as u64) as usize;
        &items[idx]
    }

    /// Choose a random index weighted by the given weights.
    ///
    /// Each element's probability is proportional to its weight.
    ///
    /// # Panics
    ///
    /// Panics if `weights` is empty or all weights are zero.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub fn weighted_index(&mut self, weights: &[u64]) -> usize {
        assert!(!weights.is_empty(), "weighted_index: empty weights");
        let total: u64 = weights.iter().sum();
        assert!(total > 0, "weighted_index: all weights are zero");

        let threshold = self.next_u64() % total;
        let mut cumulative: u64 = 0;
        for (idx, &weight) in weights.iter().enumerate() {
            cumulative += weight;
            if threshold < cumulative {
                return idx;
            }
        }
        weights.len() - 1
    }

    /// Shuffle a slice in place using Fisher-Yates.
    #[allow(clippy::cast_possible_truncation)]
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        let len = items.len();
        if len <= 1 {
            return;
        }
        for i in (1..len).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }

    /// Generate a random `bool` (50/50).
    #[must_use]
    pub fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// Generate a deterministic UUID v4 (random) from PRNG state.
    #[must_use]
    pub fn next_uuid(&mut self) -> u128 {
        let hi = self.next_u64();
        let lo = self.next_u64();
        let mut uuid = (u128::from(hi) << 64) | u128::from(lo);
        // Set version 4 and variant bits
        uuid &= !(0xF << 76); // clear version nibble
        uuid |= 0x4 << 76; // version = 4
        uuid &= !(0x3 << 62); // clear variant bits
        uuid |= 0x2 << 62; // variant = 10
        uuid
    }

    /// Get the current internal state (for serialization/checkpoint).
    #[must_use]
    pub fn state(&self) -> u64 {
        self.state
    }

    /// Restore from a previously saved state.
    #[must_use]
    pub fn from_state(state: u64) -> Self {
        Self { state }
    }
}

impl fmt::Debug for SplitMix64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SplitMix64(state=0x{:016x})", self.state)
    }
}

impl fmt::Display for SplitMix64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SplitMix64(seed={})", self.state)
    }
}

// ===========================================================================
// Fork hierarchy
// ===========================================================================

/// Named fork components for the DST framework.
///
/// The fork order is deterministic: forking in enum order always
/// produces the same sub-PRNG assignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ForkComponent {
    /// Operation generator (client commands).
    Operations,
    /// Fault injection decisions.
    Faults,
    /// Network simulator (message delivery ordering).
    Network,
    /// Storage simulator (I/O latency, corruption).
    Storage,
    /// Cooperative task scheduler.
    Scheduler,
    /// Clock jitter.
    Clock,
    /// Test data generation (names, IDs, UUIDs).
    Data,
}

impl fmt::Display for ForkComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Operations => write!(f, "operations"),
            Self::Faults => write!(f, "faults"),
            Self::Network => write!(f, "network"),
            Self::Storage => write!(f, "storage"),
            Self::Scheduler => write!(f, "scheduler"),
            Self::Clock => write!(f, "clock"),
            Self::Data => write!(f, "data"),
        }
    }
}

/// Fork a root PRNG into all DST components.
///
/// Always forks in the canonical order defined by `ForkComponent` enum
/// order, so two invocations with the same root produce identical sub-PRNGs.
#[must_use]
pub fn fork_components(root: &mut SplitMix64) -> ForkSet {
    ForkSet {
        operations: root.fork(),
        faults: root.fork(),
        network: root.fork(),
        storage: root.fork(),
        scheduler: root.fork(),
        clock: root.fork(),
        data: root.fork(),
    }
}

/// Complete set of forked sub-PRNGs for all DST components.
#[derive(Debug, Clone)]
pub struct ForkSet {
    /// Operation generator PRNG.
    pub operations: SplitMix64,
    /// Fault injection PRNG.
    pub faults: SplitMix64,
    /// Network simulator PRNG.
    pub network: SplitMix64,
    /// Storage simulator PRNG.
    pub storage: SplitMix64,
    /// Scheduler PRNG.
    pub scheduler: SplitMix64,
    /// Clock jitter PRNG.
    pub clock: SplitMix64,
    /// Test data generation PRNG.
    pub data: SplitMix64,
}

impl ForkSet {
    /// Get a mutable reference to the PRNG for a given component.
    pub fn get_mut(&mut self, component: ForkComponent) -> &mut SplitMix64 {
        match component {
            ForkComponent::Operations => &mut self.operations,
            ForkComponent::Faults => &mut self.faults,
            ForkComponent::Network => &mut self.network,
            ForkComponent::Storage => &mut self.storage,
            ForkComponent::Scheduler => &mut self.scheduler,
            ForkComponent::Clock => &mut self.clock,
            ForkComponent::Data => &mut self.data,
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    // -----------------------------------------------------------------------
    // Determinism tests
    // -----------------------------------------------------------------------

    #[test]
    fn same_seed_same_sequence() {
        let mut rng1 = SplitMix64::new(42);
        let mut rng2 = SplitMix64::new(42);

        for _ in 0..1000 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn different_seeds_different_sequences() {
        let mut rng1 = SplitMix64::new(1);
        let mut rng2 = SplitMix64::new(2);

        let seq1: Vec<u64> = (0..100).map(|_| rng1.next_u64()).collect();
        let seq2: Vec<u64> = (0..100).map(|_| rng2.next_u64()).collect();

        assert_ne!(seq1, seq2);
    }

    #[test]
    fn zero_seed_works() {
        let mut rng = SplitMix64::new(0);
        let val = rng.next_u64();
        assert_ne!(val, 0);
    }

    #[test]
    fn max_seed_works() {
        let mut rng = SplitMix64::new(u64::MAX);
        let _ = rng.next_u64();
    }

    // -----------------------------------------------------------------------
    // Fork hierarchy tests
    // -----------------------------------------------------------------------

    #[test]
    fn fork_produces_independent_streams() {
        let mut parent = SplitMix64::new(12345);
        let mut child_a = parent.fork();
        let mut child_b = parent.fork();

        let seq_a: Vec<u64> = (0..100).map(|_| child_a.next_u64()).collect();
        let seq_b: Vec<u64> = (0..100).map(|_| child_b.next_u64()).collect();

        assert_ne!(seq_a, seq_b, "Forked streams should be independent");
    }

    #[test]
    fn fork_deterministic_given_parent_state() {
        let mut parent1 = SplitMix64::new(99999);
        let mut parent2 = SplitMix64::new(99999);

        let child1 = parent1.fork();
        let child2 = parent2.fork();

        assert_eq!(child1.state(), child2.state());
    }

    #[test]
    fn fork_order_deterministic() {
        let mut first_parent = SplitMix64::new(777);
        let first_child_alpha = first_parent.fork();
        let first_child_beta = first_parent.fork();

        let mut second_parent = SplitMix64::new(777);
        let second_child_alpha = second_parent.fork();
        let second_child_beta = second_parent.fork();

        assert_eq!(first_child_alpha.state(), second_child_alpha.state());
        assert_eq!(first_child_beta.state(), second_child_beta.state());
    }

    #[test]
    fn deep_fork_chain() {
        let mut rng = SplitMix64::new(1);
        for _ in 0..100 {
            rng = rng.fork();
        }
        let val = rng.next_u64();
        assert_ne!(val, 0);
    }

    #[test]
    fn fork_components_deterministic() {
        let mut root1 = SplitMix64::new(42);
        let set1 = fork_components(&mut root1);

        let mut root2 = SplitMix64::new(42);
        let set2 = fork_components(&mut root2);

        assert_eq!(set1.operations.state(), set2.operations.state());
        assert_eq!(set1.faults.state(), set2.faults.state());
        assert_eq!(set1.network.state(), set2.network.state());
        assert_eq!(set1.storage.state(), set2.storage.state());
        assert_eq!(set1.scheduler.state(), set2.scheduler.state());
        assert_eq!(set1.clock.state(), set2.clock.state());
        assert_eq!(set1.data.state(), set2.data.state());
    }

    #[test]
    fn fork_components_all_independent() {
        let mut root = SplitMix64::new(42);
        let set = fork_components(&mut root);

        let states = [
            set.operations.state(),
            set.faults.state(),
            set.network.state(),
            set.storage.state(),
            set.scheduler.state(),
            set.clock.state(),
            set.data.state(),
        ];

        let mut unique = states.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            states.len(),
            "All component PRNGs should have unique states"
        );
    }

    #[test]
    fn fork_set_get_mut() {
        let mut root = SplitMix64::new(42);
        let mut set = fork_components(&mut root);

        let ops_state = set.operations.state();
        let ops_via_get = set.get_mut(ForkComponent::Operations).state();
        assert_eq!(ops_state, ops_via_get);
    }

    // -----------------------------------------------------------------------
    // Utility method tests
    // -----------------------------------------------------------------------

    #[test]
    fn chance_always_true_at_1() {
        let mut rng = SplitMix64::new(42);
        for _ in 0..100 {
            assert!(rng.chance(1.0));
        }
    }

    #[test]
    fn chance_always_false_at_0() {
        let mut rng = SplitMix64::new(42);
        for _ in 0..100 {
            assert!(!rng.chance(0.0));
        }
    }

    #[test]
    fn chance_statistical_accuracy() {
        let mut rng = SplitMix64::new(42);
        let n = 100_000_u64;
        let probability = 0.3;
        let count = (0..n).filter(|_| rng.chance(probability)).count();

        #[allow(clippy::cast_precision_loss)]
        let observed = count as f64 / n as f64;
        assert!(
            (observed - probability).abs() < 0.02,
            "chance(0.3) should be ~0.30, got {observed:.4}"
        );
    }

    #[test]
    fn next_f64_in_range() {
        let mut rng = SplitMix64::new(42);
        for _ in 0..10_000 {
            let val = rng.next_f64();
            assert!((0.0..1.0).contains(&val), "next_f64 out of [0,1): {val}");
        }
    }

    #[test]
    fn range_basic() {
        let mut rng = SplitMix64::new(42);
        for _ in 0..10_000 {
            let val = rng.range(10, 20);
            assert!(
                (10..=20).contains(&val),
                "range(10,20) out of bounds: {val}"
            );
        }
    }

    #[test]
    fn range_single_value() {
        let mut rng = SplitMix64::new(42);
        for _ in 0..100 {
            assert_eq!(rng.range(5, 5), 5);
        }
    }

    #[test]
    fn range_i64_basic() {
        let mut rng = SplitMix64::new(42);
        for _ in 0..10_000 {
            let val = rng.range_i64(-10, 10);
            assert!(
                (-10..=10).contains(&val),
                "range_i64(-10,10) out of bounds: {val}"
            );
        }
    }

    #[test]
    fn range_i64_negative() {
        let mut rng = SplitMix64::new(42);
        for _ in 0..10_000 {
            let val = rng.range_i64(-100, -50);
            assert!(
                (-100..=-50).contains(&val),
                "range_i64(-100,-50) out of bounds: {val}"
            );
        }
    }

    #[test]
    fn choose_returns_element() {
        let mut rng = SplitMix64::new(42);
        let items = [10, 20, 30, 40, 50];

        for _ in 0..100 {
            let chosen = rng.choose(&items);
            assert!(items.contains(chosen));
        }
    }

    #[test]
    fn choose_single_element() {
        let mut rng = SplitMix64::new(42);
        let items = [99];
        assert_eq!(*rng.choose(&items), 99);
    }

    #[test]
    fn weighted_index_basic() {
        let mut rng = SplitMix64::new(42);
        let weights = [70, 20, 10];
        let mut counts = [0_u32; 3];

        for _ in 0..100_000 {
            counts[rng.weighted_index(&weights)] += 1;
        }

        #[allow(clippy::cast_precision_loss)]
        let frac_0 = f64::from(counts[0]) / 100_000.0;
        assert!(
            (frac_0 - 0.70).abs() < 0.02,
            "weighted_index: index 0 should be ~70%, got {frac_0:.4}"
        );
    }

    #[test]
    fn weighted_index_single_weight() {
        let mut rng = SplitMix64::new(42);
        assert_eq!(rng.weighted_index(&[100]), 0);
    }

    #[test]
    fn shuffle_deterministic() {
        let mut rng1 = SplitMix64::new(42);
        let mut rng2 = SplitMix64::new(42);

        let mut arr1 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut arr2 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        rng1.shuffle(&mut arr1);
        rng2.shuffle(&mut arr2);

        assert_eq!(arr1, arr2);
    }

    #[test]
    fn shuffle_actually_permutes() {
        let mut rng = SplitMix64::new(42);
        let original = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut arr = original;
        rng.shuffle(&mut arr);

        assert_ne!(arr, original);
    }

    #[test]
    fn shuffle_preserves_elements() {
        let mut rng = SplitMix64::new(42);
        let mut arr = [1, 2, 3, 4, 5];
        rng.shuffle(&mut arr);

        let mut sorted = arr;
        sorted.sort_unstable();
        assert_eq!(sorted, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn shuffle_empty_and_single() {
        let mut rng = SplitMix64::new(42);
        let mut empty: Vec<u32> = vec![];
        rng.shuffle(&mut empty);
        assert!(empty.is_empty());

        let mut single = [1];
        rng.shuffle(&mut single);
        assert_eq!(single, [1]);
    }

    #[test]
    fn next_bool_roughly_balanced() {
        let mut rng = SplitMix64::new(42);
        let n = 100_000_u64;
        let trues = (0..n).filter(|_| rng.next_bool()).count();

        #[allow(clippy::cast_precision_loss)]
        let frac = trues as f64 / n as f64;
        assert!(
            (frac - 0.5).abs() < 0.02,
            "next_bool should be ~50/50, got {frac:.4}"
        );
    }

    #[test]
    fn next_uuid_format() {
        let mut rng = SplitMix64::new(42);
        let uuid = rng.next_uuid();

        let version = (uuid >> 76) & 0xF;
        assert_eq!(version, 4, "UUID version should be 4");

        let variant = (uuid >> 62) & 0x3;
        assert_eq!(variant, 2, "UUID variant should be 2 (10 binary)");
    }

    #[test]
    fn next_uuid_unique() {
        let mut rng = SplitMix64::new(42);
        let mut uuids: Vec<u128> = (0..1000).map(|_| rng.next_uuid()).collect();
        uuids.sort_unstable();
        uuids.dedup();
        assert_eq!(uuids.len(), 1000, "All 1000 UUIDs should be unique");
    }

    // -----------------------------------------------------------------------
    // State serialization tests
    // -----------------------------------------------------------------------

    #[test]
    fn state_roundtrip() {
        let mut rng = SplitMix64::new(42);
        for _ in 0..50 {
            let _ = rng.next_u64();
        }

        let saved = rng.state();
        let mut restored = SplitMix64::from_state(saved);

        for _ in 0..100 {
            assert_eq!(rng.next_u64(), restored.next_u64());
        }
    }

    #[test]
    fn state_checkpoint_resume() {
        let mut rng = SplitMix64::new(12345);
        let mut before: Vec<u64> = Vec::new();

        for _ in 0..10 {
            before.push(rng.next_u64());
        }

        let checkpoint = rng.state();

        let after_original: Vec<u64> = (0..10).map(|_| rng.next_u64()).collect();

        let mut resumed = SplitMix64::from_state(checkpoint);
        let after_resumed: Vec<u64> = (0..10).map(|_| resumed.next_u64()).collect();

        assert_eq!(after_original, after_resumed);
    }

    // -----------------------------------------------------------------------
    // Statistical quality tests
    // -----------------------------------------------------------------------

    #[test]
    fn chi_squared_uniformity_u64_buckets() {
        let mut rng = SplitMix64::new(42);
        let n = 1_000_000_u64;
        let num_buckets = 100;
        let mut buckets = vec![0_u64; num_buckets];

        for _ in 0..n {
            #[allow(clippy::cast_possible_truncation)]
            let idx = (rng.next_u64() % num_buckets as u64) as usize;
            buckets[idx] += 1;
        }

        #[allow(clippy::cast_precision_loss)]
        let expected = n as f64 / num_buckets as f64;
        let chi_sq: f64 = buckets
            .iter()
            .map(|&observed| {
                #[allow(clippy::cast_precision_loss)]
                let diff = observed as f64 - expected;
                diff * diff / expected
            })
            .sum();

        assert!(
            chi_sq < 150.0,
            "Chi-squared uniformity test failed: chi_sq={chi_sq:.2} (expected < 150)"
        );
    }

    #[test]
    fn range_distribution_uniform() {
        let mut rng = SplitMix64::new(42);
        let n = 100_000_u64;
        let min_val = 0_u64;
        let max_val = 9;
        let mut counts = BTreeMap::new();

        for _ in 0..n {
            let val = rng.range(min_val, max_val);
            *counts.entry(val).or_insert(0_u32) += 1;
        }

        #[allow(clippy::cast_precision_loss)]
        let expected = n as f64 / 10.0;
        for val in min_val..=max_val {
            let count = counts.get(&val).copied().unwrap_or(0);
            #[allow(clippy::cast_precision_loss)]
            let frac = f64::from(count) / expected;
            assert!(
                (frac - 1.0).abs() < 0.05,
                "range({min_val},{max_val}) value {val} has count {count}, expected ~{expected:.0}"
            );
        }
    }

    #[test]
    fn no_obvious_correlation_between_consecutive() {
        let mut rng = SplitMix64::new(42);
        let n = 10_000_u64;
        let mut values: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();

        let increases = values.windows(2).filter(|w| w[0] < w[1]).count();

        #[allow(clippy::cast_precision_loss)]
        let frac = increases as f64 / (n - 1) as f64;
        assert!(
            (frac - 0.5).abs() < 0.03,
            "Consecutive increase fraction should be ~0.5, got {frac:.4}"
        );

        values.sort_unstable();
        values.dedup();
        let unique_count = values.len();
        #[allow(clippy::cast_possible_truncation)]
        let expected_count = n as usize;
        assert_eq!(
            unique_count, expected_count,
            "Should have no duplicates in 10K values"
        );
    }

    // -----------------------------------------------------------------------
    // Display and debug tests
    // -----------------------------------------------------------------------

    #[test]
    fn display_format() {
        let rng = SplitMix64::new(42);
        let display = format!("{rng}");
        assert!(display.contains("42"));
    }

    #[test]
    fn debug_format() {
        let rng = SplitMix64::new(42);
        let debug = format!("{rng:?}");
        assert!(debug.contains("SplitMix64"));
        assert!(debug.contains("0x"));
    }

    #[test]
    fn fork_component_display() {
        assert_eq!(format!("{}", ForkComponent::Operations), "operations");
        assert_eq!(format!("{}", ForkComponent::Faults), "faults");
        assert_eq!(format!("{}", ForkComponent::Network), "network");
        assert_eq!(format!("{}", ForkComponent::Storage), "storage");
        assert_eq!(format!("{}", ForkComponent::Scheduler), "scheduler");
        assert_eq!(format!("{}", ForkComponent::Clock), "clock");
        assert_eq!(format!("{}", ForkComponent::Data), "data");
    }

    // -----------------------------------------------------------------------
    // Performance test
    // -----------------------------------------------------------------------

    #[test]
    fn performance_generates_millions_quickly() {
        let mut rng = SplitMix64::new(42);
        let n = 10_000_000_u64;

        let start = std::time::Instant::now();
        let mut checksum: u64 = 0;
        for _ in 0..n {
            checksum = checksum.wrapping_add(rng.next_u64());
        }
        let elapsed = start.elapsed();

        assert_ne!(checksum, 0);

        #[allow(clippy::cast_precision_loss)]
        let rate = n as f64 / elapsed.as_secs_f64();
        assert!(
            rate > 50_000_000.0,
            "PRNG should exceed 50M numbers/sec, got {rate:.0}/sec"
        );
    }
}
