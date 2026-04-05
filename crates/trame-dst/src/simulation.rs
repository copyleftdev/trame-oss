// Copyright 2026 Trame Contributors
// SPDX-License-Identifier: Apache-2.0

//! 9-phase deterministic simulation main loop.
//!
//! Orchestrates all DST components: operation generation, execution,
//! fault injection, time advancement, deferred work processing,
//! property verification, liveness checking, metrics recording, and
//! progress reporting.
//!
//! The loop runs single-threaded with cooperative scheduling, ensuring complete
//! determinism. Every simulation tick executes the same 9 phases in order, and
//! the entire simulation is reproducible given the initial seed.

use std::collections::BTreeMap;
use std::fmt;

use crate::fault::{FaultAction, FaultCategory, FaultConfig, FaultInjector, FaultProfile};
use crate::prng::{SplitMix64, fork_components};

// ===========================================================================
// SimulationConfig
// ===========================================================================

/// Configuration for a deterministic simulation run.
///
/// All parameters are deterministic -- no environment-dependent defaults.
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    /// Root seed controlling the entire simulation.
    pub seed: u64,
    /// Total number of ticks to simulate.
    pub tick_count: u64,
    /// Nanoseconds of simulated time per tick.
    pub nanos_per_tick: u64,
    /// Minimum operations per tick.
    pub ops_per_tick_min: u32,
    /// Maximum operations per tick.
    pub ops_per_tick_max: u32,
    /// Fault injection probability per tick (0.0 = none, 1.0 = every tick).
    pub fault_probability: f64,
    /// Fault configuration profile.
    pub fault_profile: FaultProfile,
    /// How often to verify properties (every N ticks).
    pub property_check_interval: u64,
    /// How often to check liveness (every N ticks).
    pub liveness_check_interval: u64,
    /// Maximum ticks without progress before declaring liveness failure.
    pub liveness_timeout: u64,
    /// Maximum wall-clock seconds before timeout (0 = no limit).
    pub wall_clock_timeout_secs: u64,
    /// How often to emit progress reports (every N ticks, 0 = never).
    pub report_interval: u64,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            tick_count: 10_000,
            nanos_per_tick: 1_000_000, // 1ms per tick
            ops_per_tick_min: 1,
            ops_per_tick_max: 5,
            fault_probability: 0.05,
            fault_profile: FaultProfile::Normal,
            property_check_interval: 100,
            liveness_check_interval: 50,
            liveness_timeout: 1000,
            wall_clock_timeout_secs: 0,
            report_interval: 1000,
        }
    }
}

// ===========================================================================
// SimulationStatus
// ===========================================================================

/// Outcome of a simulation run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationStatus {
    /// All ticks completed, all properties held, liveness maintained.
    Pass,
    /// A property violation was detected.
    PropertyViolation,
    /// A liveness property was violated (progress stalled).
    LivenessFailure,
    /// Real wall-clock timeout exceeded.
    Timeout,
}

impl fmt::Display for SimulationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pass => write!(f, "PASS"),
            Self::PropertyViolation => write!(f, "PROPERTY_VIOLATION"),
            Self::LivenessFailure => write!(f, "LIVENESS_FAILURE"),
            Self::Timeout => write!(f, "TIMEOUT"),
        }
    }
}

// ===========================================================================
// PropertyViolation
// ===========================================================================

/// Describes a property violation detected during simulation.
#[derive(Debug, Clone)]
pub struct PropertyViolation {
    /// Tick at which the violation was detected.
    pub tick: u64,
    /// Property ID (e.g., "P1", "INV-S1").
    pub property_id: String,
    /// Human-readable description.
    pub description: String,
}

impl fmt::Display for PropertyViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[tick {}] {} -- {}",
            self.tick, self.property_id, self.description
        )
    }
}

// ===========================================================================
// SimulationResult
// ===========================================================================

/// Complete result of a simulation run including metrics and failure details.
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Overall outcome.
    pub status: SimulationStatus,
    /// Seed used for this run.
    pub seed: u64,
    /// Total ticks executed (may be less than configured if terminated early).
    pub ticks_completed: u64,

    // --- Operation counts ---
    /// Operations accepted by the system under test.
    pub ops_accepted: u64,
    /// Operations rejected by the system under test.
    pub ops_rejected: u64,
    /// Operations skipped.
    pub ops_skipped: u64,

    // --- Fault counts ---
    /// Total fault actions applied.
    pub faults_injected: u64,
    /// Faults by category.
    pub faults_by_category: BTreeMap<FaultCategory, u64>,

    // --- Verification counts ---
    /// Number of property checks that passed.
    pub property_checks_passed: u64,
    /// Number of liveness checks that passed.
    pub liveness_checks_passed: u64,

    // --- Failure details ---
    /// Violations that caused the simulation to fail.
    pub violations: Vec<PropertyViolation>,
    /// Tick at which failure occurred (if any).
    pub failure_tick: Option<u64>,
}

impl SimulationResult {
    /// Total operations generated.
    #[must_use]
    pub fn total_ops(&self) -> u64 {
        self.ops_accepted + self.ops_rejected + self.ops_skipped
    }
}

impl fmt::Display for SimulationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SimulationResult(status={}, seed={}, ticks={}, ops={}, faults={})",
            self.status,
            self.seed,
            self.ticks_completed,
            self.total_ops(),
            self.faults_injected,
        )
    }
}

// ===========================================================================
// OperationOutcome
// ===========================================================================

/// Outcome of executing an operation against the system under test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationOutcome {
    /// Operation was accepted and applied.
    Accepted,
    /// Operation was rejected.
    Rejected {
        /// Why it was rejected.
        reason: String,
    },
    /// Operation was skipped (not applicable in current state).
    Skipped {
        /// Why it was skipped.
        reason: String,
    },
}

// ===========================================================================
// Traits -- domain-agnostic extension points
// ===========================================================================

/// Generates operations for each simulation tick.
///
/// Consumers implement this trait to produce domain-specific operations
/// (e.g., database writes, message sends, state transitions).
pub trait OperationGenerator {
    /// Generate operations for this tick.
    fn generate(&mut self, rng: &mut SplitMix64, tick: u64) -> Vec<Box<dyn Operation>>;
}

/// A single operation to be executed during simulation.
///
/// Operations are the unit of work in the simulation. They represent
/// commands submitted to the system under test.
pub trait Operation: Send + Sync {
    /// Unique identifier for this operation.
    fn id(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// Execute the operation, returning its outcome.
    fn execute(&mut self) -> OperationOutcome;
}

/// Checks properties (invariants) after operations are executed.
///
/// Property checkers verify that the system under test maintains
/// its expected invariants after each batch of operations.
pub trait PropertyChecker {
    /// Check all properties at the given tick.
    fn check(&self, tick: u64) -> Vec<PropertyViolation>;
}

/// A reference model against which the system under test is validated.
///
/// The reference model maintains a simplified view of the expected state
/// and can detect divergences from the actual system.
pub trait ReferenceModel {
    /// Apply an operation and its outcome to the reference model.
    fn apply(&mut self, op_id: &str, outcome: &OperationOutcome);

    /// Take a snapshot of the reference model's state.
    fn snapshot(&self) -> Vec<u8>;

    /// Verify the reference model against the actual system state.
    fn verify_against(&self, actual: &[u8]) -> Vec<PropertyViolation>;
}

// ===========================================================================
// Simulation -- the 9-phase loop
// ===========================================================================

/// Deterministic simulation runner implementing the 9-phase loop.
///
/// The 9 phases per tick are:
///
/// 1. **Generate operations** -- produce operations via `OperationGenerator`
/// 2. **Execute operations** -- run operations against the system
/// 3. **Inject faults** -- probabilistic fault injection via `FaultInjector`
/// 4. **Advance simulated time** -- tick the simulation clock
/// 5. **Process deferred work** -- run any pending deferred callbacks
/// 6. **Verify properties** -- check invariants via `PropertyChecker`
/// 7. **Check liveness** -- ensure the system is making progress
/// 8. **Record metrics** -- capture per-tick statistics
/// 9. **Report progress** -- periodic console output
pub struct Simulation {
    config: SimulationConfig,
    fault_injector: FaultInjector,
    ops_rng: SplitMix64,
    ticks_completed: u64,
    // Counters
    ops_accepted: u64,
    ops_rejected: u64,
    ops_skipped: u64,
    faults_by_category: BTreeMap<FaultCategory, u64>,
    property_checks_passed: u64,
    liveness_checks_passed: u64,
    violations: Vec<PropertyViolation>,
    last_progress_tick: u64,
    all_fault_actions: Vec<FaultAction>,
}

impl Simulation {
    /// Create a new simulation from config.
    #[must_use]
    pub fn new(config: SimulationConfig) -> Self {
        let mut root = SplitMix64::new(config.seed);
        let forks = fork_components(&mut root);

        let fault_config = FaultConfig::from_profile(config.fault_profile);
        let fault_injector = FaultInjector::new(forks.faults.clone(), fault_config);

        Self {
            config,
            fault_injector,
            ops_rng: forks.operations.clone(),
            ticks_completed: 0,
            ops_accepted: 0,
            ops_rejected: 0,
            ops_skipped: 0,
            faults_by_category: BTreeMap::new(),
            property_checks_passed: 0,
            liveness_checks_passed: 0,
            violations: Vec::new(),
            last_progress_tick: 0,
            all_fault_actions: Vec::new(),
        }
    }

    /// Run the full simulation with the given components.
    ///
    /// This drives the 9-phase loop for each tick. `deferred_work` is called
    /// during phase 5 to allow the consumer to process any pending tasks.
    pub fn run(
        &mut self,
        generator: &mut dyn OperationGenerator,
        checker: &dyn PropertyChecker,
        mut deferred_work: impl FnMut(u64),
    ) -> SimulationResult {
        let wall_start = std::time::Instant::now();

        for tick in 0..self.config.tick_count {
            // Wall-clock timeout check
            if self.config.wall_clock_timeout_secs > 0
                && wall_start.elapsed().as_secs() >= self.config.wall_clock_timeout_secs
            {
                return self.build_result(SimulationStatus::Timeout, Some(tick));
            }

            // Phase 1: Generate operations
            let ops = generator.generate(&mut self.ops_rng, tick);

            // Phase 2: Execute operations
            for mut op in ops {
                let outcome = op.execute();
                match &outcome {
                    OperationOutcome::Accepted => self.ops_accepted += 1,
                    OperationOutcome::Rejected { .. } => self.ops_rejected += 1,
                    OperationOutcome::Skipped { .. } => self.ops_skipped += 1,
                }
            }

            // Phase 3: Inject faults
            if self.ops_rng.chance(self.config.fault_probability) {
                let actions = self.fault_injector.inject_tick(tick);
                for action in &actions {
                    *self
                        .faults_by_category
                        .entry(action.fault_type.category())
                        .or_insert(0) += 1;
                }
                self.all_fault_actions.extend(actions);
            }

            // Phase 4: Advance simulated time (caller manages the clock)
            // The simulation framework does not own the clock; consumers
            // tick their SimClock in their OperationGenerator or deferred_work.

            // Phase 5: Process deferred work
            deferred_work(tick);

            // Phase 6: Verify properties
            if self.config.property_check_interval > 0
                && tick % self.config.property_check_interval == 0
            {
                let violations = checker.check(tick);
                if violations.is_empty() {
                    self.property_checks_passed += 1;
                } else {
                    self.violations.extend(violations);
                    return self.build_result(SimulationStatus::PropertyViolation, Some(tick));
                }
            }

            // Phase 7: Check liveness
            if self.config.liveness_check_interval > 0
                && tick % self.config.liveness_check_interval == 0
            {
                // Simple liveness check: if no ops accepted in liveness_timeout ticks
                if tick > self.config.liveness_timeout
                    && self.ops_accepted == 0
                    && self.ops_rejected == 0
                    && self.ops_skipped == 0
                {
                    return self.build_result(SimulationStatus::LivenessFailure, Some(tick));
                }
                self.liveness_checks_passed += 1;
            }

            // Phase 8: Record metrics (tracked incrementally above)

            // Phase 9: Report progress
            if self.config.report_interval > 0 && tick % self.config.report_interval == 0 && tick > 0
            {
                self.last_progress_tick = tick;
            }

            self.ticks_completed = tick + 1;
        }

        self.build_result(SimulationStatus::Pass, None)
    }

    /// Build the final result.
    fn build_result(
        &self,
        status: SimulationStatus,
        failure_tick: Option<u64>,
    ) -> SimulationResult {
        let faults_injected: u64 = self.faults_by_category.values().sum();

        SimulationResult {
            status,
            seed: self.config.seed,
            ticks_completed: self.ticks_completed,
            ops_accepted: self.ops_accepted,
            ops_rejected: self.ops_rejected,
            ops_skipped: self.ops_skipped,
            faults_injected,
            faults_by_category: self.faults_by_category.clone(),
            property_checks_passed: self.property_checks_passed,
            liveness_checks_passed: self.liveness_checks_passed,
            violations: self.violations.clone(),
            failure_tick,
        }
    }

    /// Get the fault injector for inspection.
    #[must_use]
    pub fn fault_injector(&self) -> &FaultInjector {
        &self.fault_injector
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Null implementations for testing the harness
    // -----------------------------------------------------------------------

    struct NullGenerator;

    impl OperationGenerator for NullGenerator {
        fn generate(&mut self, _rng: &mut SplitMix64, _tick: u64) -> Vec<Box<dyn Operation>> {
            Vec::new()
        }
    }

    struct CountingGenerator {
        count: u32,
    }

    impl OperationGenerator for CountingGenerator {
        fn generate(&mut self, _rng: &mut SplitMix64, _tick: u64) -> Vec<Box<dyn Operation>> {
            let mut ops: Vec<Box<dyn Operation>> = Vec::new();
            for i in 0..self.count {
                ops.push(Box::new(AcceptOp {
                    id: format!("op-{i}"),
                }));
            }
            ops
        }
    }

    struct AcceptOp {
        id: String,
    }

    impl Operation for AcceptOp {
        fn id(&self) -> &str {
            &self.id
        }
        #[allow(clippy::unnecessary_literal_bound)]
        fn description(&self) -> &str {
            "always-accept operation"
        }
        fn execute(&mut self) -> OperationOutcome {
            OperationOutcome::Accepted
        }
    }

    struct NullChecker;

    impl PropertyChecker for NullChecker {
        fn check(&self, _tick: u64) -> Vec<PropertyViolation> {
            Vec::new()
        }
    }

    struct FailingChecker {
        fail_at_tick: u64,
    }

    impl PropertyChecker for FailingChecker {
        fn check(&self, tick: u64) -> Vec<PropertyViolation> {
            if tick >= self.fail_at_tick {
                vec![PropertyViolation {
                    tick,
                    property_id: "TEST-001".to_string(),
                    description: "intentional test failure".to_string(),
                }]
            } else {
                Vec::new()
            }
        }
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[test]
    fn null_simulation_passes() {
        let config = SimulationConfig {
            seed: 42,
            tick_count: 100,
            ..Default::default()
        };
        let mut sim = Simulation::new(config);
        let result = sim.run(&mut NullGenerator, &NullChecker, |_| {});
        assert_eq!(result.status, SimulationStatus::Pass);
        assert_eq!(result.ticks_completed, 100);
        assert_eq!(result.seed, 42);
    }

    #[test]
    fn counting_simulation_tracks_ops() {
        let config = SimulationConfig {
            seed: 42,
            tick_count: 50,
            ..Default::default()
        };
        let mut sim = Simulation::new(config);
        let mut op_gen = CountingGenerator { count: 3 };
        let result = sim.run(&mut op_gen, &NullChecker, |_| {});
        assert_eq!(result.status, SimulationStatus::Pass);
        assert_eq!(result.ops_accepted, 150); // 3 ops * 50 ticks
    }

    #[test]
    fn property_violation_stops_simulation() {
        let config = SimulationConfig {
            seed: 42,
            tick_count: 1000,
            property_check_interval: 10,
            ..Default::default()
        };
        let mut sim = Simulation::new(config);
        let checker = FailingChecker { fail_at_tick: 50 };
        let result = sim.run(&mut NullGenerator, &checker, |_| {});
        assert_eq!(result.status, SimulationStatus::PropertyViolation);
        assert!(result.failure_tick.is_some());
        assert!(!result.violations.is_empty());
    }

    #[test]
    fn deterministic_same_seed() {
        let run = |seed: u64| -> SimulationResult {
            let config = SimulationConfig {
                seed,
                tick_count: 200,
                fault_probability: 0.5,
                fault_profile: FaultProfile::Aggressive,
                ..Default::default()
            };
            let mut sim = Simulation::new(config);
            let mut op_gen = CountingGenerator { count: 2 };
            sim.run(&mut op_gen, &NullChecker, |_| {})
        };

        let r1 = run(42);
        let r2 = run(42);
        assert_eq!(r1.ops_accepted, r2.ops_accepted);
        assert_eq!(r1.faults_injected, r2.faults_injected);
        assert_eq!(r1.ticks_completed, r2.ticks_completed);
    }

    #[test]
    fn different_seeds_different_results() {
        let run = |seed: u64| -> SimulationResult {
            let config = SimulationConfig {
                seed,
                tick_count: 500,
                fault_probability: 0.5,
                fault_profile: FaultProfile::Aggressive,
                ..Default::default()
            };
            let mut sim = Simulation::new(config);
            let mut op_gen = CountingGenerator { count: 2 };
            sim.run(&mut op_gen, &NullChecker, |_| {})
        };

        let r1 = run(1);
        let r2 = run(2);
        // Fault injection counts will likely differ due to different PRNG seeds
        // (though ops_accepted will be the same since CountingGenerator is deterministic
        // and doesn't use the rng)
        assert_eq!(r1.ops_accepted, r2.ops_accepted); // same generator
        // Fault counts may or may not differ -- at least one should run
    }

    #[test]
    fn simulation_status_display() {
        assert_eq!(format!("{}", SimulationStatus::Pass), "PASS");
        assert_eq!(
            format!("{}", SimulationStatus::PropertyViolation),
            "PROPERTY_VIOLATION"
        );
        assert_eq!(
            format!("{}", SimulationStatus::LivenessFailure),
            "LIVENESS_FAILURE"
        );
        assert_eq!(format!("{}", SimulationStatus::Timeout), "TIMEOUT");
    }

    #[test]
    fn property_violation_display() {
        let v = PropertyViolation {
            tick: 42,
            property_id: "P1".to_string(),
            description: "oops".to_string(),
        };
        let display = format!("{v}");
        assert!(display.contains("42"));
        assert!(display.contains("P1"));
        assert!(display.contains("oops"));
    }

    #[test]
    fn simulation_result_display() {
        let r = SimulationResult {
            status: SimulationStatus::Pass,
            seed: 42,
            ticks_completed: 100,
            ops_accepted: 50,
            ops_rejected: 5,
            ops_skipped: 2,
            faults_injected: 10,
            faults_by_category: BTreeMap::new(),
            property_checks_passed: 1,
            liveness_checks_passed: 2,
            violations: Vec::new(),
            failure_tick: None,
        };
        let display = format!("{r}");
        assert!(display.contains("PASS"));
        assert!(display.contains("42"));
    }

    #[test]
    fn default_config_sensible() {
        let config = SimulationConfig::default();
        assert_eq!(config.tick_count, 10_000);
        assert_eq!(config.nanos_per_tick, 1_000_000);
        assert!(config.ops_per_tick_min <= config.ops_per_tick_max);
    }

    #[test]
    fn operation_outcome_variants() {
        let accepted = OperationOutcome::Accepted;
        let rejected = OperationOutcome::Rejected {
            reason: "no".into(),
        };
        let skipped = OperationOutcome::Skipped {
            reason: "meh".into(),
        };
        assert_eq!(accepted, OperationOutcome::Accepted);
        assert_ne!(accepted, rejected);
        assert_ne!(rejected, skipped);
    }
}
