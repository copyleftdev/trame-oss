// Copyright 2026 Trame Contributors
// SPDX-License-Identifier: Apache-2.0

//! Fault injection engine for deterministic simulation testing.
//!
//! Supports 32 fault types across storage, network, process, clock, and
//! composite categories. All decisions derive from [`SplitMix64`] PRNG for
//! full seed-based reproducibility.
//!
//! The engine manages fault scheduling, active fault tracking with duration/expiry,
//! cooldown enforcement, and fault history logging.

use std::collections::BTreeMap;
use std::fmt;

use crate::prng::SplitMix64;

// ===========================================================================
// FaultType -- 32 fault types across 5 categories
// ===========================================================================

/// Enumeration of all 32 injectable fault types.
///
/// Organized into storage (10), network (8), process (4), clock (3),
/// and composite (7) categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FaultType {
    // --- Storage faults (S-001 through S-010) ---
    /// S-001: `write()` returns I/O error; data unchanged.
    StorageWriteFailure,
    /// S-002: `read()` returns I/O error; data on disk intact but inaccessible.
    StorageReadFailure,
    /// S-003: `fsync()` returns error; data may or may not be durable.
    StorageFsyncFailure,
    /// S-004: Only partial bytes persisted on write.
    StoragePartialWrite,
    /// S-005: Random bit flipped in data returned by read.
    StorageBitFlip,
    /// S-006: Operation completes but with 10x-100x normal latency.
    StorageLatencySpike,
    /// S-007: `write()` returns ENOSPC. No data written.
    StorageDiskFull,
    /// S-008: Sector partially written (torn at byte boundary within sector).
    StorageTornPage,
    /// S-009: `read()` returns superseded data version.
    StorageStaleRead,
    /// S-010: All unfsynced writes lost; storage truncated to last fsync.
    StoragePowerLoss,

    // --- Network faults (N-001 through N-008) ---
    /// N-001: Message silently discarded.
    NetworkMessageDrop,
    /// N-002: Message buffered and delivered after N ticks.
    NetworkMessageDelay,
    /// N-003: Messages delivered out of send order.
    NetworkMessageReorder,
    /// N-004: Same message delivered K times identically.
    NetworkMessageDuplicate,
    /// N-005: Random bytes in payload flipped.
    NetworkMessageCorrupt,
    /// N-006: Complete bidirectional partition between two replica sets.
    NetworkPartitionFull,
    /// N-007: Unidirectional partition (A->B works, B->A dropped).
    NetworkPartitionAsymmetric,
    /// N-008: Messages dropped with probability p.
    NetworkPartitionPartial,

    // --- Process faults (P-001 through P-004) ---
    /// P-001: Replica killed instantly; all in-memory state lost.
    ProcessCrash,
    /// P-002: Previously crashed replica restarts from durable state.
    ProcessRestart,
    /// P-003: Replica stops processing for N ticks; state preserved.
    ProcessPause,
    /// P-004: Replica runs at reduced speed (simulates CPU contention).
    ProcessSlow,

    // --- Clock faults (C-001 through C-003) ---
    /// C-001: Clock drifts relative to others by configurable amount.
    ClockSkew,
    /// C-002: Clock suddenly advances by N nanoseconds.
    ClockJumpForward,
    /// C-003: Clock stops advancing for duration.
    ClockStall,

    // --- Composite faults (X-001 through X-007) ---
    /// X-001: Crash + discard unflushed writes + restart.
    CompositePowerLoss,
    /// X-002: Fault on one replica triggers faults on others.
    CompositeCascadingFailure,
    /// X-003: Replica produces incorrect but plausible responses.
    CompositeByzantine,
    /// X-004: Network partition + leader on each side.
    CompositeSplitBrain,
    /// X-005: Sequential restart of all replicas with overlap.
    CompositeRollingRestart,
    /// X-006: Crash + clear storage + restart (new node).
    CompositeDiskReplacement,
    /// X-007: Multiple simultaneous minor faults.
    CompositeDegradedMode,
}

impl FaultType {
    /// Returns all 32 fault types in canonical order.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::StorageWriteFailure,
            Self::StorageReadFailure,
            Self::StorageFsyncFailure,
            Self::StoragePartialWrite,
            Self::StorageBitFlip,
            Self::StorageLatencySpike,
            Self::StorageDiskFull,
            Self::StorageTornPage,
            Self::StorageStaleRead,
            Self::StoragePowerLoss,
            Self::NetworkMessageDrop,
            Self::NetworkMessageDelay,
            Self::NetworkMessageReorder,
            Self::NetworkMessageDuplicate,
            Self::NetworkMessageCorrupt,
            Self::NetworkPartitionFull,
            Self::NetworkPartitionAsymmetric,
            Self::NetworkPartitionPartial,
            Self::ProcessCrash,
            Self::ProcessRestart,
            Self::ProcessPause,
            Self::ProcessSlow,
            Self::ClockSkew,
            Self::ClockJumpForward,
            Self::ClockStall,
            Self::CompositePowerLoss,
            Self::CompositeCascadingFailure,
            Self::CompositeByzantine,
            Self::CompositeSplitBrain,
            Self::CompositeRollingRestart,
            Self::CompositeDiskReplacement,
            Self::CompositeDegradedMode,
        ]
    }

    /// Returns the category of this fault type.
    #[must_use]
    pub fn category(&self) -> FaultCategory {
        match self {
            Self::StorageWriteFailure
            | Self::StorageReadFailure
            | Self::StorageFsyncFailure
            | Self::StoragePartialWrite
            | Self::StorageBitFlip
            | Self::StorageLatencySpike
            | Self::StorageDiskFull
            | Self::StorageTornPage
            | Self::StorageStaleRead
            | Self::StoragePowerLoss => FaultCategory::Storage,

            Self::NetworkMessageDrop
            | Self::NetworkMessageDelay
            | Self::NetworkMessageReorder
            | Self::NetworkMessageDuplicate
            | Self::NetworkMessageCorrupt
            | Self::NetworkPartitionFull
            | Self::NetworkPartitionAsymmetric
            | Self::NetworkPartitionPartial => FaultCategory::Network,

            Self::ProcessCrash | Self::ProcessRestart | Self::ProcessPause | Self::ProcessSlow => {
                FaultCategory::Process
            }

            Self::ClockSkew | Self::ClockJumpForward | Self::ClockStall => FaultCategory::Clock,

            Self::CompositePowerLoss
            | Self::CompositeCascadingFailure
            | Self::CompositeByzantine
            | Self::CompositeSplitBrain
            | Self::CompositeRollingRestart
            | Self::CompositeDiskReplacement
            | Self::CompositeDegradedMode => FaultCategory::Composite,
        }
    }

    /// Returns the spec ID for this fault type (e.g., "S-001").
    #[must_use]
    pub fn spec_id(&self) -> &'static str {
        match self {
            Self::StorageWriteFailure => "S-001",
            Self::StorageReadFailure => "S-002",
            Self::StorageFsyncFailure => "S-003",
            Self::StoragePartialWrite => "S-004",
            Self::StorageBitFlip => "S-005",
            Self::StorageLatencySpike => "S-006",
            Self::StorageDiskFull => "S-007",
            Self::StorageTornPage => "S-008",
            Self::StorageStaleRead => "S-009",
            Self::StoragePowerLoss => "S-010",
            Self::NetworkMessageDrop => "N-001",
            Self::NetworkMessageDelay => "N-002",
            Self::NetworkMessageReorder => "N-003",
            Self::NetworkMessageDuplicate => "N-004",
            Self::NetworkMessageCorrupt => "N-005",
            Self::NetworkPartitionFull => "N-006",
            Self::NetworkPartitionAsymmetric => "N-007",
            Self::NetworkPartitionPartial => "N-008",
            Self::ProcessCrash => "P-001",
            Self::ProcessRestart => "P-002",
            Self::ProcessPause => "P-003",
            Self::ProcessSlow => "P-004",
            Self::ClockSkew => "C-001",
            Self::ClockJumpForward => "C-002",
            Self::ClockStall => "C-003",
            Self::CompositePowerLoss => "X-001",
            Self::CompositeCascadingFailure => "X-002",
            Self::CompositeByzantine => "X-003",
            Self::CompositeSplitBrain => "X-004",
            Self::CompositeRollingRestart => "X-005",
            Self::CompositeDiskReplacement => "X-006",
            Self::CompositeDegradedMode => "X-007",
        }
    }

    /// Default cooldown in ticks between injections of this fault type.
    #[must_use]
    pub fn default_cooldown(&self) -> u64 {
        match self {
            Self::StorageWriteFailure
            | Self::StorageReadFailure
            | Self::StorageBitFlip
            | Self::StorageStaleRead => 10,

            Self::StoragePartialWrite | Self::StorageTornPage | Self::ProcessSlow => 20,

            Self::StorageFsyncFailure
            | Self::StorageDiskFull
            | Self::ProcessRestart
            | Self::ProcessPause => 50,

            Self::StorageLatencySpike
            | Self::NetworkMessageDrop
            | Self::NetworkMessageDelay
            | Self::NetworkMessageReorder
            | Self::NetworkMessageDuplicate
            | Self::NetworkMessageCorrupt => 5,

            Self::StoragePowerLoss
            | Self::CompositePowerLoss
            | Self::CompositeCascadingFailure
            | Self::CompositeDiskReplacement => 500,

            Self::NetworkPartitionFull | Self::ClockJumpForward | Self::CompositeByzantine => 200,

            Self::NetworkPartitionAsymmetric => 150,

            Self::NetworkPartitionPartial
            | Self::ProcessCrash
            | Self::ClockSkew
            | Self::ClockStall
            | Self::CompositeDegradedMode => 100,

            Self::CompositeSplitBrain => 300,
            Self::CompositeRollingRestart => 1000,
        }
    }

    /// Whether this fault has a duration (vs. instantaneous).
    #[must_use]
    pub fn has_duration(&self) -> bool {
        matches!(
            self,
            Self::StorageLatencySpike
                | Self::NetworkPartitionFull
                | Self::NetworkPartitionAsymmetric
                | Self::NetworkPartitionPartial
                | Self::ProcessPause
                | Self::ProcessSlow
                | Self::ClockSkew
                | Self::ClockStall
                | Self::CompositeSplitBrain
                | Self::CompositeDegradedMode
        )
    }
}

impl fmt::Display for FaultType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:?})", self.spec_id(), self)
    }
}

// ===========================================================================
// FaultCategory
// ===========================================================================

/// High-level category grouping fault types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FaultCategory {
    /// Storage I/O faults.
    Storage,
    /// Network communication faults.
    Network,
    /// Process lifecycle faults.
    Process,
    /// Clock and time faults.
    Clock,
    /// Multi-component composite faults.
    Composite,
}

impl FaultCategory {
    /// Returns all categories in canonical order.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::Storage,
            Self::Network,
            Self::Process,
            Self::Clock,
            Self::Composite,
        ]
    }
}

impl fmt::Display for FaultCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage => write!(f, "storage"),
            Self::Network => write!(f, "network"),
            Self::Process => write!(f, "process"),
            Self::Clock => write!(f, "clock"),
            Self::Composite => write!(f, "composite"),
        }
    }
}

// ===========================================================================
// FaultTarget -- what the fault affects
// ===========================================================================

/// Target of a fault injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultTarget {
    /// Fault targets a specific replica.
    Replica(u16),
    /// Fault targets a specific network link.
    Link { from: u16, to: u16 },
    /// Fault targets a network topology (partition).
    Partition { side_a: Vec<u16>, side_b: Vec<u16> },
    /// Fault affects all replicas.
    Global,
}

// ===========================================================================
// FaultParameters -- type-specific parameters
// ===========================================================================

/// Type-specific parameters for an injected fault.
#[derive(Debug, Clone, PartialEq)]
pub enum FaultParameters {
    /// No additional parameters needed.
    None,
    /// Latency multiplier for slow operations.
    LatencyMultiplier(u32),
    /// Bit flip location.
    BitFlip { byte_offset: u64, bit_position: u8 },
    /// Partial write: bytes actually written.
    PartialWrite { bytes_written: u64 },
    /// Torn page: tear offset within sector.
    TornPage { tear_offset: u16 },
    /// Stale read: how many versions behind.
    StaleRead { staleness_versions: u32 },
    /// Network delay in ticks.
    Delay { ticks: u64 },
    /// Message duplication count.
    Duplicate { count: u32 },
    /// Clock skew amount in nanoseconds.
    ClockOffset { nanos: i64 },
    /// Number of corrupt bytes.
    CorruptBytes { count: u32 },
    /// Drop probability for partial partition.
    DropProbability { probability: f64 },
    /// Slow process speed reduction factor.
    SpeedFactor { factor: u32 },
}

// ===========================================================================
// FaultId -- unique fault identifier
// ===========================================================================

/// Unique identifier for a fault injection event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaultId(pub u64);

impl fmt::Display for FaultId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fault-{}", self.0)
    }
}

// ===========================================================================
// FaultAction -- what the engine tells the simulation to do
// ===========================================================================

/// Action returned by the fault injector for the simulation to execute.
#[derive(Debug, Clone, PartialEq)]
pub struct FaultAction {
    /// Unique ID for this fault injection.
    pub id: FaultId,
    /// Type of fault being injected.
    pub fault_type: FaultType,
    /// What the fault targets.
    pub target: FaultTarget,
    /// Type-specific parameters.
    pub parameters: FaultParameters,
    /// Duration in ticks (None for instantaneous).
    pub duration_ticks: Option<u64>,
}

// ===========================================================================
// ActiveFault -- currently-active fault with expiry tracking
// ===========================================================================

/// A fault that is currently active with optional expiration.
#[derive(Debug, Clone)]
pub struct ActiveFault {
    /// Unique fault identifier.
    pub id: FaultId,
    /// The type of fault.
    pub fault_type: FaultType,
    /// What the fault targets.
    pub target: FaultTarget,
    /// Tick when this fault was injected.
    pub started_tick: u64,
    /// Tick when this fault expires (None = instantaneous, already applied).
    pub expires_tick: Option<u64>,
    /// Type-specific parameters.
    pub parameters: FaultParameters,
}

impl ActiveFault {
    /// Returns `true` if the fault has expired at the given tick.
    #[must_use]
    pub fn is_expired(&self, tick: u64) -> bool {
        self.expires_tick.is_some_and(|exp| tick >= exp)
    }
}

// ===========================================================================
// FaultRecord -- historical log entry
// ===========================================================================

/// Record of a fault that was injected (for history/debugging).
#[derive(Debug, Clone)]
pub struct FaultRecord {
    /// Unique fault identifier.
    pub id: FaultId,
    /// The type of fault.
    pub fault_type: FaultType,
    /// What the fault targeted.
    pub target: FaultTarget,
    /// Tick when injected.
    pub tick: u64,
    /// Type-specific parameters.
    pub parameters: FaultParameters,
    /// How long the fault lasted (None = instantaneous).
    pub duration_ticks: Option<u64>,
}

// ===========================================================================
// FaultProfile -- predefined configurations
// ===========================================================================

/// Predefined fault configuration profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultProfile {
    /// No faults injected (baseline testing).
    None,
    /// 0.1x probabilities, 1 fault max.
    Gentle,
    /// Default probabilities.
    Normal,
    /// 10x probabilities, 5 faults max.
    Aggressive,
    /// 50x storage faults.
    StorageStress,
    /// 50x network faults.
    NetworkChaos,
    /// 100x crash probability.
    CrashHappy,
}

// ===========================================================================
// FaultConfig -- per-type probabilities and limits
// ===========================================================================

/// Configuration for the fault injection engine.
///
/// Per-type probabilities control how often each fault fires.
/// Limits and duration ranges bound the fault injection behavior.
#[derive(Debug, Clone)]
pub struct FaultConfig {
    /// Per-fault-type injection probability.
    pub probabilities: BTreeMap<FaultType, f64>,
    /// Per-fault-type cooldown in ticks (overrides defaults).
    pub cooldowns: BTreeMap<FaultType, u64>,
    /// Maximum number of concurrently active faults.
    pub max_active_faults: u32,
    /// Maximum concurrent network partitions.
    pub max_active_partitions: u32,
    /// Minimum healthy replicas (safety floor).
    pub min_healthy_replicas: u32,
    /// Partition duration range in ticks `[min, max)`.
    pub partition_duration_min: u64,
    /// Maximum partition duration in ticks.
    pub partition_duration_max: u64,
    /// Minimum pause duration in ticks.
    pub pause_duration_min: u64,
    /// Maximum pause duration in ticks.
    pub pause_duration_max: u64,
    /// Minimum restart delay in ticks.
    pub restart_delay_min: u64,
    /// Maximum restart delay in ticks.
    pub restart_delay_max: u64,
    /// Minimum latency spike multiplier.
    pub latency_multiplier_min: u32,
    /// Maximum latency spike multiplier.
    pub latency_multiplier_max: u32,
    /// Minimum network delay in ticks.
    pub network_delay_min: u64,
    /// Maximum network delay in ticks.
    pub network_delay_max: u64,
    /// Minimum clock skew in nanoseconds.
    pub clock_skew_min_nanos: i64,
    /// Maximum clock skew in nanoseconds.
    pub clock_skew_max_nanos: i64,
    /// Minimum clock jump in nanoseconds.
    pub clock_jump_min_nanos: u64,
    /// Maximum clock jump in nanoseconds.
    pub clock_jump_max_nanos: u64,
}

impl FaultConfig {
    /// Create a config from a predefined profile.
    #[must_use]
    pub fn from_profile(profile: FaultProfile) -> Self {
        let multiplier = match profile {
            FaultProfile::None => 0.0,
            FaultProfile::Gentle => 0.1,
            FaultProfile::Normal
            | FaultProfile::StorageStress
            | FaultProfile::NetworkChaos
            | FaultProfile::CrashHappy => 1.0,
            FaultProfile::Aggressive => 10.0,
        };

        let mut probabilities = BTreeMap::new();

        // Storage fault defaults
        let storage_mult = match profile {
            FaultProfile::StorageStress => 50.0,
            _ => multiplier,
        };
        probabilities.insert(FaultType::StorageWriteFailure, 0.001 * storage_mult);
        probabilities.insert(FaultType::StorageReadFailure, 0.000_5 * storage_mult);
        probabilities.insert(FaultType::StorageFsyncFailure, 0.001 * storage_mult);
        probabilities.insert(FaultType::StoragePartialWrite, 0.000_2 * storage_mult);
        probabilities.insert(FaultType::StorageBitFlip, 0.000_1 * storage_mult);
        probabilities.insert(FaultType::StorageLatencySpike, 0.005 * storage_mult);
        probabilities.insert(FaultType::StorageDiskFull, 0.000_1 * storage_mult);
        probabilities.insert(FaultType::StorageTornPage, 0.000_2 * storage_mult);
        probabilities.insert(FaultType::StorageStaleRead, 0.000_1 * storage_mult);
        probabilities.insert(FaultType::StoragePowerLoss, 0.000_05 * storage_mult);

        // Network fault defaults
        let network_mult = match profile {
            FaultProfile::NetworkChaos => 50.0,
            _ => multiplier,
        };
        probabilities.insert(FaultType::NetworkMessageDrop, 0.01 * network_mult);
        probabilities.insert(FaultType::NetworkMessageDelay, 0.05 * network_mult);
        probabilities.insert(FaultType::NetworkMessageReorder, 0.02 * network_mult);
        probabilities.insert(FaultType::NetworkMessageDuplicate, 0.005 * network_mult);
        probabilities.insert(FaultType::NetworkMessageCorrupt, 0.001 * network_mult);
        probabilities.insert(FaultType::NetworkPartitionFull, 0.001 * network_mult);
        probabilities.insert(
            FaultType::NetworkPartitionAsymmetric,
            0.000_5 * network_mult,
        );
        probabilities.insert(FaultType::NetworkPartitionPartial, 0.000_5 * network_mult);

        // Process fault defaults
        let process_mult = match profile {
            FaultProfile::CrashHappy => 100.0,
            _ => multiplier,
        };
        probabilities.insert(FaultType::ProcessCrash, 0.000_5 * process_mult);
        probabilities.insert(FaultType::ProcessRestart, 0.0); // triggered by crashed state
        probabilities.insert(FaultType::ProcessPause, 0.002 * process_mult);
        probabilities.insert(FaultType::ProcessSlow, 0.001 * process_mult);

        // Clock fault defaults
        probabilities.insert(FaultType::ClockSkew, 0.001 * multiplier);
        probabilities.insert(FaultType::ClockJumpForward, 0.000_2 * multiplier);
        probabilities.insert(FaultType::ClockStall, 0.000_1 * multiplier);

        // Composite fault defaults
        probabilities.insert(FaultType::CompositePowerLoss, 0.000_05 * multiplier);
        probabilities.insert(FaultType::CompositeCascadingFailure, 0.000_05 * multiplier);
        probabilities.insert(FaultType::CompositeByzantine, 0.000_01 * multiplier);
        probabilities.insert(FaultType::CompositeSplitBrain, 0.000_05 * multiplier);
        probabilities.insert(FaultType::CompositeRollingRestart, 0.000_1 * multiplier);
        probabilities.insert(FaultType::CompositeDiskReplacement, 0.000_05 * multiplier);
        probabilities.insert(FaultType::CompositeDegradedMode, 0.000_1 * multiplier);

        let max_active = match profile {
            FaultProfile::None => 0,
            FaultProfile::Gentle => 1,
            FaultProfile::Aggressive => 5,
            FaultProfile::Normal
            | FaultProfile::StorageStress
            | FaultProfile::NetworkChaos
            | FaultProfile::CrashHappy => 3,
        };

        Self {
            probabilities,
            cooldowns: BTreeMap::new(),
            max_active_faults: max_active,
            max_active_partitions: 1,
            min_healthy_replicas: 1,
            partition_duration_min: 50,
            partition_duration_max: 500,
            pause_duration_min: 10,
            pause_duration_max: 200,
            restart_delay_min: 5,
            restart_delay_max: 100,
            latency_multiplier_min: 10,
            latency_multiplier_max: 100,
            network_delay_min: 1,
            network_delay_max: 1000,
            clock_skew_min_nanos: -5_000_000_000, // -5s
            clock_skew_max_nanos: 5_000_000_000,  // +5s
            clock_jump_min_nanos: 100_000_000,    // 100ms
            clock_jump_max_nanos: 30_000_000_000, // 30s
        }
    }

    /// Get the probability for a specific fault type.
    #[must_use]
    pub fn probability(&self, fault_type: FaultType) -> f64 {
        self.probabilities.get(&fault_type).copied().unwrap_or(0.0)
    }

    /// Get the cooldown for a fault type (custom override or default).
    #[must_use]
    pub fn cooldown(&self, fault_type: FaultType) -> u64 {
        self.cooldowns
            .get(&fault_type)
            .copied()
            .unwrap_or_else(|| fault_type.default_cooldown())
    }
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self::from_profile(FaultProfile::Normal)
    }
}

// ===========================================================================
// FaultMetrics -- aggregate statistics
// ===========================================================================

/// Aggregate metrics from the fault injector.
#[derive(Debug, Clone, Default)]
pub struct FaultMetrics {
    /// Total faults injected by category.
    pub by_category: BTreeMap<FaultCategory, u64>,
    /// Total faults injected by type.
    pub by_type: BTreeMap<FaultType, u64>,
    /// Currently active fault count.
    pub active_count: usize,
    /// Total faults injected.
    pub total_injected: u64,
    /// Total faults that were skipped due to cooldown.
    pub skipped_cooldown: u64,
    /// Total faults skipped due to max active limit.
    pub skipped_max_active: u64,
}

// ===========================================================================
// FaultInjector -- the engine
// ===========================================================================

/// Deterministic fault injection engine.
///
/// All decisions derive from the PRNG seed. Given the same seed and the same
/// sequence of calls, the engine produces identical fault sequences.
pub struct FaultInjector {
    prng: SplitMix64,
    config: FaultConfig,
    active_faults: Vec<ActiveFault>,
    fault_history: Vec<FaultRecord>,
    cooldown_until: BTreeMap<FaultType, u64>,
    tick: u64,
    next_fault_id: u64,
    // Metrics
    total_injected: u64,
    skipped_cooldown: u64,
    skipped_max_active: u64,
    injected_by_type: BTreeMap<FaultType, u64>,
}

impl FaultInjector {
    /// Create a new fault injector with the given PRNG and configuration.
    #[must_use]
    pub fn new(prng: SplitMix64, config: FaultConfig) -> Self {
        Self {
            prng,
            config,
            active_faults: Vec::new(),
            fault_history: Vec::new(),
            cooldown_until: BTreeMap::new(),
            tick: 0,
            next_fault_id: 0,
            total_injected: 0,
            skipped_cooldown: 0,
            skipped_max_active: 0,
            injected_by_type: BTreeMap::new(),
        }
    }

    /// Current simulation tick.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Reference to the current configuration.
    #[must_use]
    pub fn config(&self) -> &FaultConfig {
        &self.config
    }

    /// Mutable reference to the configuration.
    pub fn config_mut(&mut self) -> &mut FaultConfig {
        &mut self.config
    }

    /// Currently active faults.
    #[must_use]
    pub fn active_faults(&self) -> &[ActiveFault] {
        &self.active_faults
    }

    /// Number of currently active faults.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active_faults.len()
    }

    /// Fault history (all faults that have been injected).
    #[must_use]
    pub fn history(&self) -> &[FaultRecord] {
        &self.fault_history
    }

    /// Aggregate metrics.
    #[must_use]
    pub fn metrics(&self) -> FaultMetrics {
        let mut by_category = BTreeMap::new();
        for (ft, &count) in &self.injected_by_type {
            *by_category.entry(ft.category()).or_insert(0) += count;
        }
        FaultMetrics {
            by_category,
            by_type: self.injected_by_type.clone(),
            active_count: self.active_faults.len(),
            total_injected: self.total_injected,
            skipped_cooldown: self.skipped_cooldown,
            skipped_max_active: self.skipped_max_active,
        }
    }

    /// Advance to the given tick, expire completed faults, and evaluate
    /// each fault type for injection.
    ///
    /// Returns the list of new fault actions the simulation should execute.
    pub fn inject_tick(&mut self, tick: u64) -> Vec<FaultAction> {
        self.tick = tick;

        // Phase 1: Expire completed faults
        self.expire_faults(tick);

        // Phase 2: Evaluate each fault type for injection
        let mut actions = Vec::new();

        for &fault_type in FaultType::all() {
            if let Some(action) = self.try_inject(fault_type) {
                actions.push(action);
            }
        }

        actions
    }

    /// Expire faults whose duration has elapsed at the given tick.
    pub fn expire_faults(&mut self, tick: u64) {
        let expired: Vec<ActiveFault> = self
            .active_faults
            .iter()
            .filter(|f| f.is_expired(tick))
            .cloned()
            .collect();

        self.active_faults.retain(|f| !f.is_expired(tick));

        for fault in expired {
            let duration = fault.expires_tick.map(|exp| exp - fault.started_tick);
            self.fault_history.push(FaultRecord {
                id: fault.id,
                fault_type: fault.fault_type,
                target: fault.target,
                tick: fault.started_tick,
                parameters: fault.parameters,
                duration_ticks: duration,
            });
        }
    }

    // -- Internal helpers -------------------------------------------------

    /// Try to inject a specific fault type. Returns an action if injection
    /// succeeds, or `None` if skipped.
    fn try_inject(&mut self, fault_type: FaultType) -> Option<FaultAction> {
        let p = self.config.probability(fault_type);
        if p <= 0.0 {
            return None;
        }

        // Check cooldown
        if let Some(&until) = self.cooldown_until.get(&fault_type) {
            if self.tick < until {
                self.skipped_cooldown += 1;
                return None;
            }
        }

        // Check max active faults (for duration-based faults)
        #[allow(clippy::cast_possible_truncation)]
        let max = self.config.max_active_faults as usize;
        if fault_type.has_duration() && self.active_faults.len() >= max {
            self.skipped_max_active += 1;
            return None;
        }

        // PRNG roll
        if !self.prng.chance(p) {
            return None;
        }

        // Set cooldown
        let cooldown = self.config.cooldown(fault_type);
        self.cooldown_until.insert(fault_type, self.tick + cooldown);

        // Record metrics
        self.total_injected += 1;
        *self.injected_by_type.entry(fault_type).or_insert(0) += 1;

        // Build the action
        let id = self.allocate_fault_id();
        let duration_ticks = if fault_type.has_duration() {
            let dur = self.prng.range(
                self.config.partition_duration_min,
                self.config.partition_duration_max,
            );
            Some(dur)
        } else {
            None
        };

        let action = FaultAction {
            id,
            fault_type,
            target: FaultTarget::Global,
            parameters: FaultParameters::None,
            duration_ticks,
        };

        // Track active faults
        if let Some(dur) = duration_ticks {
            self.active_faults.push(ActiveFault {
                id: action.id,
                fault_type: action.fault_type,
                target: action.target.clone(),
                started_tick: self.tick,
                expires_tick: Some(self.tick + dur),
                parameters: action.parameters.clone(),
            });
        }

        Some(action)
    }

    fn allocate_fault_id(&mut self) -> FaultId {
        let id = FaultId(self.next_fault_id);
        self.next_fault_id += 1;
        id
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_injector(seed: u64, profile: FaultProfile) -> FaultInjector {
        let prng = SplitMix64::new(seed);
        let config = FaultConfig::from_profile(profile);
        FaultInjector::new(prng, config)
    }

    // -----------------------------------------------------------------------
    // Fault type enumeration
    // -----------------------------------------------------------------------

    #[test]
    fn all_32_fault_types() {
        assert_eq!(FaultType::all().len(), 32);
    }

    #[test]
    fn all_fault_types_have_spec_ids() {
        for ft in FaultType::all() {
            let id = ft.spec_id();
            assert!(!id.is_empty(), "Fault type {ft:?} has no spec ID");
        }
    }

    #[test]
    fn all_fault_types_have_categories() {
        for ft in FaultType::all() {
            let _ = ft.category();
        }
    }

    #[test]
    fn category_counts() {
        let storage = FaultType::all()
            .iter()
            .filter(|ft| ft.category() == FaultCategory::Storage)
            .count();
        let network = FaultType::all()
            .iter()
            .filter(|ft| ft.category() == FaultCategory::Network)
            .count();
        let process = FaultType::all()
            .iter()
            .filter(|ft| ft.category() == FaultCategory::Process)
            .count();
        let clock = FaultType::all()
            .iter()
            .filter(|ft| ft.category() == FaultCategory::Clock)
            .count();
        let composite = FaultType::all()
            .iter()
            .filter(|ft| ft.category() == FaultCategory::Composite)
            .count();

        assert_eq!(storage, 10);
        assert_eq!(network, 8);
        assert_eq!(process, 4);
        assert_eq!(clock, 3);
        assert_eq!(composite, 7);
    }

    #[test]
    fn five_fault_categories() {
        assert_eq!(FaultCategory::all().len(), 5);
    }

    // -----------------------------------------------------------------------
    // Profile construction
    // -----------------------------------------------------------------------

    #[test]
    fn none_profile_zero_probabilities() {
        let config = FaultConfig::from_profile(FaultProfile::None);
        for ft in FaultType::all() {
            let p = config.probability(*ft);
            assert!(
                p.abs() < f64::EPSILON,
                "FaultProfile::None should have 0 probability for {ft:?}, got {p}"
            );
        }
    }

    #[test]
    fn none_profile_zero_max_active() {
        let config = FaultConfig::from_profile(FaultProfile::None);
        assert_eq!(config.max_active_faults, 0);
    }

    #[test]
    fn aggressive_profile_higher_probabilities() {
        let normal = FaultConfig::from_profile(FaultProfile::Normal);
        let aggressive = FaultConfig::from_profile(FaultProfile::Aggressive);

        for ft in FaultType::all() {
            let normal_p = normal.probability(*ft);
            let aggressive_p = aggressive.probability(*ft);
            assert!(
                aggressive_p >= normal_p,
                "Aggressive should be >= Normal for {ft:?}: {aggressive_p} < {normal_p}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Deterministic injection
    // -----------------------------------------------------------------------

    #[test]
    fn deterministic_injection_same_seed() {
        let run = |seed: u64| {
            let mut injector = make_injector(seed, FaultProfile::Aggressive);
            let mut all_actions = Vec::new();
            for tick in 0..1000 {
                let actions = injector.inject_tick(tick);
                for a in &actions {
                    all_actions.push((tick, a.fault_type, a.id));
                }
            }
            all_actions
        };

        let run1 = run(42);
        let run2 = run(42);
        assert_eq!(run1, run2, "Same seed should produce same fault sequence");
    }

    #[test]
    fn different_seeds_different_faults() {
        let run = |seed: u64| {
            let mut injector = make_injector(seed, FaultProfile::Aggressive);
            let mut types = Vec::new();
            for tick in 0..1000 {
                let actions = injector.inject_tick(tick);
                for a in actions {
                    types.push(a.fault_type);
                }
            }
            types
        };

        let run1 = run(1);
        let run2 = run(2);
        assert_ne!(
            run1, run2,
            "Different seeds should produce different faults"
        );
    }

    // -----------------------------------------------------------------------
    // Cooldown enforcement
    // -----------------------------------------------------------------------

    #[test]
    fn cooldown_prevents_rapid_re_injection() {
        let prng = SplitMix64::new(42);
        // Force a single fault type with probability 1.0
        let mut config = FaultConfig::from_profile(FaultProfile::None);
        config.max_active_faults = 100;
        config
            .probabilities
            .insert(FaultType::StorageWriteFailure, 1.0);
        config.cooldowns.insert(FaultType::StorageWriteFailure, 50);

        let mut injector = FaultInjector::new(prng, config);

        // First tick should inject
        let actions = injector.inject_tick(0);
        let first_count = actions
            .iter()
            .filter(|a| a.fault_type == FaultType::StorageWriteFailure)
            .count();
        assert_eq!(first_count, 1, "Should inject on first tick");

        // Ticks 1-49 should be on cooldown
        for tick in 1..50 {
            let actions = injector.inject_tick(tick);
            let count = actions
                .iter()
                .filter(|a| a.fault_type == FaultType::StorageWriteFailure)
                .count();
            assert_eq!(count, 0, "Should be on cooldown at tick {tick}");
        }

        // Tick 50 should inject again
        let actions = injector.inject_tick(50);
        let count = actions
            .iter()
            .filter(|a| a.fault_type == FaultType::StorageWriteFailure)
            .count();
        assert_eq!(count, 1, "Should inject again after cooldown at tick 50");
    }

    // -----------------------------------------------------------------------
    // Active fault tracking and expiry
    // -----------------------------------------------------------------------

    #[test]
    fn active_faults_expire() {
        let prng = SplitMix64::new(42);
        let mut config = FaultConfig::from_profile(FaultProfile::None);
        config.max_active_faults = 10;
        // Force a duration-based fault
        config
            .probabilities
            .insert(FaultType::NetworkPartitionFull, 1.0);
        config.partition_duration_min = 10;
        config.partition_duration_max = 10;

        let mut injector = FaultInjector::new(prng, config);

        let actions = injector.inject_tick(0);
        assert!(!actions.is_empty(), "Should inject partition");
        assert_eq!(injector.active_count(), 1);

        // Still active before expiry
        let _ = injector.inject_tick(5);
        assert!(injector.active_count() >= 1);

        // Expire at tick 10
        let _ = injector.inject_tick(10);
        // The original partition should have expired
        let originally_active_at_0 = injector
            .history()
            .iter()
            .any(|r| r.fault_type == FaultType::NetworkPartitionFull && r.tick == 0);
        assert!(
            originally_active_at_0,
            "Expired fault should appear in history"
        );
    }

    // -----------------------------------------------------------------------
    // Metrics
    // -----------------------------------------------------------------------

    #[test]
    fn metrics_track_injections() {
        let mut injector = make_injector(42, FaultProfile::Aggressive);
        for tick in 0..500 {
            let _ = injector.inject_tick(tick);
        }
        let metrics = injector.metrics();
        assert!(
            metrics.total_injected > 0,
            "Should have injected at least one fault in 500 ticks"
        );
    }

    #[test]
    fn fault_type_display() {
        let display = format!("{}", FaultType::StorageWriteFailure);
        assert!(display.contains("S-001"));
    }

    #[test]
    fn fault_category_display() {
        assert_eq!(format!("{}", FaultCategory::Storage), "storage");
        assert_eq!(format!("{}", FaultCategory::Network), "network");
        assert_eq!(format!("{}", FaultCategory::Process), "process");
        assert_eq!(format!("{}", FaultCategory::Clock), "clock");
        assert_eq!(format!("{}", FaultCategory::Composite), "composite");
    }

    #[test]
    fn fault_id_display() {
        let id = FaultId(42);
        assert_eq!(format!("{id}"), "fault-42");
    }
}
