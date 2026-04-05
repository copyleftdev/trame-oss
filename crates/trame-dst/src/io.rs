// Copyright 2026 Trame Contributors
// SPDX-License-Identifier: Apache-2.0

//! I/O trait abstractions for deterministic simulation testing.
//!
//! All system-under-test I/O passes through these traits so the DST framework
//! can inject faults and control time deterministically. No direct `std::time`,
//! `std::fs`, or `std::net` calls in simulated business logic -- only through
//! these trait implementations.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

// ===========================================================================
// ReplicaId -- unique identifier for a simulated node
// ===========================================================================

/// Unique identifier for a replica / node in the simulated cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReplicaId(pub u16);

impl fmt::Display for ReplicaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "replica-{}", self.0)
    }
}

// ===========================================================================
// Timestamp -- nanosecond-precision simulation time
// ===========================================================================

/// Nanosecond-precision timestamp used throughout the simulation.
///
/// Internally an `i64` counting nanoseconds since epoch. Negative values
/// represent times before epoch (useful for testing edge cases).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(i64);

impl Timestamp {
    /// The Unix epoch (time zero).
    pub const EPOCH: Self = Self(0);

    /// The maximum representable timestamp.
    pub const MAX: Self = Self(i64::MAX);

    /// Create a timestamp from nanoseconds since epoch.
    #[must_use]
    pub const fn from_nanos(nanos: i64) -> Self {
        Self(nanos)
    }

    /// Returns the raw nanosecond value.
    #[must_use]
    pub const fn as_nanos(self) -> i64 {
        self.0
    }

    /// Returns `true` if this is the maximum timestamp.
    #[must_use]
    pub const fn is_max(self) -> bool {
        self.0 == i64::MAX
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ns", self.0)
    }
}

// ===========================================================================
// Clock trait
// ===========================================================================

/// Abstraction over time sources.
///
/// Simulated code uses `Clock` instead of `std::time::SystemTime::now()`
/// directly, enabling deterministic simulation where time is manually advanced.
///
/// All implementations must be `Send + Sync` for multi-threaded usage.
pub trait Clock: Send + Sync {
    /// Returns the current timestamp.
    fn now(&self) -> Timestamp;

    /// Advance simulated time by `nanos` nanoseconds.
    ///
    /// For production clocks this is a no-op (real time advances on its own).
    /// For simulation clocks this is the primary time-advancement primitive.
    fn tick(&self, _nanos: u64) {}

    /// Nanoseconds elapsed since `instant`, i.e. `now() - instant`.
    ///
    /// Returns a negative value if `instant` is in the future relative to
    /// the current clock reading (useful for detecting clock skew in tests).
    fn elapsed_since(&self, instant: Timestamp) -> i64 {
        self.now().as_nanos().wrapping_sub(instant.as_nanos())
    }
}

// ===========================================================================
// RealClock
// ===========================================================================

/// Production clock backed by `std::time::SystemTime`.
#[derive(Debug, Default)]
pub struct RealClock;

impl RealClock {
    /// Create a new real clock.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Clock for RealClock {
    fn now(&self) -> Timestamp {
        let duration = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before Unix epoch");
        #[allow(clippy::cast_possible_truncation)]
        let nanos = duration.as_nanos() as i64;
        Timestamp::from_nanos(nanos)
    }
}

// ===========================================================================
// SimClock
// ===========================================================================

/// Deterministic simulation clock with manual time advancement.
///
/// Time only advances when explicitly told to via [`Clock::tick`] or
/// [`SimClock::advance`], enabling reproducible tests.
///
/// **Monotonicity guarantee**: the clock never goes backward.
///
/// Thread-safe via atomic operations.
#[derive(Debug)]
pub struct SimClock {
    nanos: AtomicI64,
}

impl SimClock {
    /// Create a new simulation clock starting at the given timestamp.
    #[must_use]
    pub fn new(start: Timestamp) -> Self {
        Self {
            nanos: AtomicI64::new(start.as_nanos()),
        }
    }

    /// Create a new simulation clock starting at epoch.
    #[must_use]
    pub fn at_epoch() -> Self {
        Self::new(Timestamp::EPOCH)
    }

    /// Advance the clock by `delta_nanos` nanoseconds.
    ///
    /// # Panics
    ///
    /// Panics if `delta_nanos` is negative (would violate monotonicity).
    pub fn advance(&self, delta_nanos: i64) {
        assert!(
            delta_nanos >= 0,
            "SimClock::advance: negative delta ({delta_nanos}ns) would violate monotonicity"
        );
        self.nanos.fetch_add(delta_nanos, Ordering::Release);
    }

    /// Set the clock to an exact timestamp.
    ///
    /// # Panics
    ///
    /// Panics if `ts` is before the current time (would violate monotonicity).
    pub fn set(&self, ts: Timestamp) {
        let new_val = ts.as_nanos();
        let current = self.nanos.load(Ordering::Acquire);
        assert!(
            new_val >= current,
            "SimClock::set: new timestamp ({new_val}ns) < current ({current}ns), \
             would violate monotonicity"
        );
        self.nanos.store(new_val, Ordering::Release);
    }

    /// Get the current internal state for checkpoint/serialization.
    #[must_use]
    pub fn state(&self) -> i64 {
        self.nanos.load(Ordering::Acquire)
    }

    /// Restore a `SimClock` from a previously saved state.
    #[must_use]
    pub fn from_state(nanos: i64) -> Self {
        Self {
            nanos: AtomicI64::new(nanos),
        }
    }
}

impl Clock for SimClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_nanos(self.nanos.load(Ordering::Acquire))
    }

    fn tick(&self, nanos: u64) {
        let delta = i64::try_from(nanos)
            .expect("SimClock::tick: duration exceeds i64::MAX nanoseconds (~292 years)");
        self.nanos.fetch_add(delta, Ordering::Release);
    }
}

// ===========================================================================
// StorageError
// ===========================================================================

/// Error type for simulated storage operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// Generic I/O error.
    Io { errno: i32, message: String },
    /// Data corruption detected on read.
    CorruptRead {
        /// Byte offset of corrupt data.
        offset: u64,
        /// Expected CRC.
        expected_crc: u32,
        /// Actual CRC.
        actual_crc: u32,
    },
    /// Storage device is full.
    DiskFull,
    /// Operation timed out.
    Timeout,
    /// Injected fsync failure.
    FsyncFailed {
        /// Human-readable description.
        message: String,
    },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { errno, message } => write!(f, "I/O error (errno {errno}): {message}"),
            Self::CorruptRead {
                offset,
                expected_crc,
                actual_crc,
            } => write!(
                f,
                "corrupt read at offset {offset}: expected CRC {expected_crc:#010x}, got {actual_crc:#010x}"
            ),
            Self::DiskFull => write!(f, "disk full"),
            Self::Timeout => write!(f, "storage operation timed out"),
            Self::FsyncFailed { message } => write!(f, "fsync failed: {message}"),
        }
    }
}

impl std::error::Error for StorageError {}

// ===========================================================================
// SimulatedStorage trait
// ===========================================================================

/// Abstraction over block-level storage for deterministic simulation.
///
/// Operations are offset-based, similar to `pread`/`pwrite` semantics.
pub trait SimulatedStorage: Send + Sync {
    /// Read `len` bytes starting at `offset`.
    fn read(&self, offset: u64, len: u32) -> Result<Vec<u8>, StorageError>;

    /// Write `data` at `offset`.
    fn write(&self, offset: u64, data: &[u8]) -> Result<(), StorageError>;

    /// Ensure all preceding writes are durable (fsync).
    fn fsync(&self) -> Result<(), StorageError>;

    /// Allocate `size` bytes, returning the starting offset.
    fn allocate(&self, size: u64) -> Result<u64, StorageError>;

    /// Simulate a crash: discard all unflushed writes.
    fn crash(&self);

    /// Recover from a crash: restore to last fsync'd state.
    fn recover(&self);
}

/// Sector size in bytes, matching typical storage hardware.
pub const SECTOR_SIZE: usize = 4096;

// ===========================================================================
// MemStorage
// ===========================================================================

/// In-memory sector-based storage for deterministic simulation testing.
///
/// Writes are immediately visible to reads (like OS page cache) but not
/// durable until [`SimulatedStorage::fsync`]. [`MemStorage::crash`] discards
/// all unflushed writes, reverting to the last fsync'd state.
pub struct MemStorage {
    /// Current working data (includes unflushed writes, visible to reads).
    data: Mutex<Vec<u8>>,
    /// Durable data (survives crash, updated on fsync).
    stable: Mutex<Vec<u8>>,
    /// Count of unflushed writes.
    unflushed_count: Mutex<u64>,
    /// Total bytes written.
    total_written: AtomicU64,
    /// Total bytes read.
    total_read: AtomicU64,
}

impl MemStorage {
    /// Create a new empty in-memory storage.
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: Mutex::new(Vec::new()),
            stable: Mutex::new(Vec::new()),
            unflushed_count: Mutex::new(0),
            total_written: AtomicU64::new(0),
            total_read: AtomicU64::new(0),
        }
    }

    /// Create a new in-memory storage pre-populated with data.
    #[must_use]
    pub fn with_data(initial: Vec<u8>) -> Self {
        Self {
            data: Mutex::new(initial.clone()),
            stable: Mutex::new(initial),
            unflushed_count: Mutex::new(0),
            total_written: AtomicU64::new(0),
            total_read: AtomicU64::new(0),
        }
    }

    /// Total bytes written since creation.
    #[must_use]
    pub fn bytes_written(&self) -> u64 {
        self.total_written.load(Ordering::Acquire)
    }

    /// Total bytes read since creation.
    #[must_use]
    pub fn bytes_read(&self) -> u64 {
        self.total_read.load(Ordering::Acquire)
    }

    /// Count of unflushed writes.
    #[must_use]
    pub fn unflushed_count(&self) -> u64 {
        *self.unflushed_count.lock().expect("lock poisoned")
    }

    /// Get a snapshot of the current data.
    #[must_use]
    pub fn snapshot(&self) -> Vec<u8> {
        self.data.lock().expect("lock poisoned").clone()
    }
}

impl Default for MemStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedStorage for MemStorage {
    fn read(&self, offset: u64, len: u32) -> Result<Vec<u8>, StorageError> {
        let data = self.data.lock().expect("lock poisoned");
        #[allow(clippy::cast_possible_truncation)]
        let start = offset as usize;
        let end = start + len as usize;
        if end > data.len() {
            // Read beyond allocated: return zeros for unallocated region
            let mut result = Vec::with_capacity(len as usize);
            if start < data.len() {
                result.extend_from_slice(&data[start..]);
            }
            result.resize(len as usize, 0);
            self.total_read.fetch_add(u64::from(len), Ordering::Release);
            return Ok(result);
        }
        let result = data[start..end].to_vec();
        self.total_read.fetch_add(u64::from(len), Ordering::Release);
        Ok(result)
    }

    fn write(&self, offset: u64, buf: &[u8]) -> Result<(), StorageError> {
        let mut data = self.data.lock().expect("lock poisoned");
        #[allow(clippy::cast_possible_truncation)]
        let start = offset as usize;
        let end = start + buf.len();
        if end > data.len() {
            data.resize(end, 0);
        }
        data[start..end].copy_from_slice(buf);
        *self.unflushed_count.lock().expect("lock poisoned") += 1;
        #[allow(clippy::cast_possible_truncation)]
        self.total_written
            .fetch_add(buf.len() as u64, Ordering::Release);
        Ok(())
    }

    fn fsync(&self) -> Result<(), StorageError> {
        let data = self.data.lock().expect("lock poisoned");
        let mut stable = self.stable.lock().expect("lock poisoned");
        (*stable).clone_from(&data);
        *self.unflushed_count.lock().expect("lock poisoned") = 0;
        Ok(())
    }

    #[allow(clippy::cast_possible_truncation)]
    fn allocate(&self, size: u64) -> Result<u64, StorageError> {
        let mut data = self.data.lock().expect("lock poisoned");
        let current_len = data.len();
        let offset = current_len as u64;
        data.resize(current_len + size as usize, 0);
        Ok(offset)
    }

    fn crash(&self) {
        let stable = self.stable.lock().expect("lock poisoned");
        let mut data = self.data.lock().expect("lock poisoned");
        (*data).clone_from(&stable);
        *self.unflushed_count.lock().expect("lock poisoned") = 0;
    }

    fn recover(&self) {
        // Recover is the same as crash for MemStorage: revert to stable.
        self.crash();
    }
}

// Compile-time check: MemStorage is Send + Sync
const _: () = {
    fn assert_send_sync<T: Send + Sync>() {}
    fn check() {
        assert_send_sync::<MemStorage>();
    }
    let _ = check;
};

// ===========================================================================
// NetworkError
// ===========================================================================

/// Error type for simulated network operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    /// Destination is unreachable (partitioned or down).
    Unreachable,
    /// Send queue is full.
    QueueFull,
    /// Operation timed out.
    Timeout,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreachable => write!(f, "destination unreachable"),
            Self::QueueFull => write!(f, "send queue full"),
            Self::Timeout => write!(f, "network operation timed out"),
        }
    }
}

impl std::error::Error for NetworkError {}

// ===========================================================================
// SimulatedNetwork trait
// ===========================================================================

/// Abstraction over network communication between replicas.
///
/// Simulated code uses `SimulatedNetwork` instead of `std::net` directly,
/// enabling deterministic simulation with partitions, delays, and reordering.
pub trait SimulatedNetwork: Send + Sync {
    /// Send a message from one replica to another.
    fn send(&self, from: ReplicaId, to: ReplicaId, message: Vec<u8>) -> Result<(), NetworkError>;

    /// Receive the next available message for a replica, if any.
    /// Returns `(sender, payload)`.
    fn recv(&self, node: ReplicaId) -> Option<(ReplicaId, Vec<u8>)>;

    /// Create a bidirectional partition between two sets of replicas.
    fn partition(&self, side_a: &[ReplicaId], side_b: &[ReplicaId]);

    /// Heal all partitions, restoring full connectivity.
    fn heal(&self);
}

// ===========================================================================
// MemNetwork
// ===========================================================================

/// All mutable state under a single lock to avoid deadlock.
struct MemNetworkInner {
    /// Ready-to-deliver messages per destination.
    ready: BTreeMap<ReplicaId, VecDeque<(ReplicaId, Vec<u8>)>>,
    /// Directed partition set: `(from, to)` pairs where messages are blocked.
    partitions: std::collections::BTreeSet<(ReplicaId, ReplicaId)>,
    /// Messages sent count.
    sent_count: u64,
    /// Messages delivered count.
    delivered_count: u64,
    /// Messages dropped count.
    dropped_count: u64,
}

/// In-memory network for deterministic simulation testing.
///
/// Messages are queued per destination. Supports network partitions.
/// With no configuration, messages are delivered immediately (zero latency).
pub struct MemNetwork {
    inner: Mutex<MemNetworkInner>,
}

impl MemNetwork {
    /// Create a new simulation network with no partitions.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(MemNetworkInner {
                ready: BTreeMap::new(),
                partitions: std::collections::BTreeSet::new(),
                sent_count: 0,
                delivered_count: 0,
                dropped_count: 0,
            }),
        }
    }

    /// Total messages sent.
    #[must_use]
    pub fn sent_count(&self) -> u64 {
        self.inner.lock().expect("lock poisoned").sent_count
    }

    /// Total messages delivered.
    #[must_use]
    pub fn delivered_count(&self) -> u64 {
        self.inner.lock().expect("lock poisoned").delivered_count
    }

    /// Total messages dropped.
    #[must_use]
    pub fn dropped_count(&self) -> u64 {
        self.inner.lock().expect("lock poisoned").dropped_count
    }
}

impl Default for MemNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedNetwork for MemNetwork {
    fn send(&self, from: ReplicaId, to: ReplicaId, message: Vec<u8>) -> Result<(), NetworkError> {
        let mut inner = self.inner.lock().expect("lock poisoned");
        inner.sent_count += 1;

        // Check if the link is partitioned
        if inner.partitions.contains(&(from, to)) {
            inner.dropped_count += 1;
            return Err(NetworkError::Unreachable);
        }

        inner
            .ready
            .entry(to)
            .or_default()
            .push_back((from, message));
        Ok(())
    }

    fn recv(&self, node: ReplicaId) -> Option<(ReplicaId, Vec<u8>)> {
        let mut inner = self.inner.lock().expect("lock poisoned");
        let msg = inner.ready.get_mut(&node)?.pop_front();
        if msg.is_some() {
            inner.delivered_count += 1;
        }
        msg
    }

    fn partition(&self, side_a: &[ReplicaId], side_b: &[ReplicaId]) {
        let mut inner = self.inner.lock().expect("lock poisoned");
        for &a in side_a {
            for &b in side_b {
                inner.partitions.insert((a, b));
                inner.partitions.insert((b, a));
            }
        }
    }

    fn heal(&self) {
        let mut inner = self.inner.lock().expect("lock poisoned");
        inner.partitions.clear();
    }
}

// Compile-time check: MemNetwork is Send + Sync
const _: () = {
    fn assert_send_sync<T: Send + Sync>() {}
    fn check() {
        assert_send_sync::<MemNetwork>();
    }
    let _ = check;
};

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Timestamp tests
    // -----------------------------------------------------------------------

    #[test]
    fn timestamp_epoch() {
        assert_eq!(Timestamp::EPOCH.as_nanos(), 0);
    }

    #[test]
    fn timestamp_max() {
        assert!(Timestamp::MAX.is_max());
        assert!(!Timestamp::EPOCH.is_max());
    }

    #[test]
    fn timestamp_display() {
        let ts = Timestamp::from_nanos(42);
        assert_eq!(format!("{ts}"), "42ns");
    }

    #[test]
    fn timestamp_ordering() {
        let a = Timestamp::from_nanos(1);
        let b = Timestamp::from_nanos(2);
        assert!(a < b);
    }

    // -----------------------------------------------------------------------
    // SimClock tests
    // -----------------------------------------------------------------------

    #[test]
    fn sim_clock_starts_at_given_time() {
        let clock = SimClock::new(Timestamp::from_nanos(1000));
        assert_eq!(clock.now(), Timestamp::from_nanos(1000));
    }

    #[test]
    fn sim_clock_advance() {
        let clock = SimClock::at_epoch();
        assert_eq!(clock.now(), Timestamp::EPOCH);
        clock.advance(500);
        assert_eq!(clock.now(), Timestamp::from_nanos(500));
        clock.advance(300);
        assert_eq!(clock.now(), Timestamp::from_nanos(800));
    }

    #[test]
    fn sim_clock_set() {
        let clock = SimClock::at_epoch();
        clock.set(Timestamp::from_nanos(9999));
        assert_eq!(clock.now(), Timestamp::from_nanos(9999));
    }

    #[test]
    fn tick_advances_simclock() {
        let clock = SimClock::at_epoch();
        clock.tick(1_000_000);
        assert_eq!(clock.now(), Timestamp::from_nanos(1_000_000));
        clock.tick(500_000);
        assert_eq!(clock.now(), Timestamp::from_nanos(1_500_000));
    }

    #[test]
    fn real_clock_returns_positive() {
        let clock = RealClock::new();
        let now = clock.now();
        assert!(now > Timestamp::EPOCH);
    }

    #[test]
    fn elapsed_since_basic() {
        let clock = SimClock::new(Timestamp::from_nanos(1000));
        let start = clock.now();
        clock.advance(500);
        assert_eq!(clock.elapsed_since(start), 500);
    }

    #[test]
    fn elapsed_since_future_is_negative() {
        let clock = SimClock::new(Timestamp::from_nanos(100));
        let future = Timestamp::from_nanos(200);
        assert_eq!(clock.elapsed_since(future), -100);
    }

    #[test]
    #[should_panic(expected = "negative delta")]
    fn advance_negative_panics() {
        let clock = SimClock::new(Timestamp::from_nanos(1000));
        clock.advance(-1);
    }

    #[test]
    #[should_panic(expected = "would violate monotonicity")]
    fn set_backward_panics() {
        let clock = SimClock::new(Timestamp::from_nanos(1000));
        clock.set(Timestamp::from_nanos(999));
    }

    #[test]
    fn state_roundtrip() {
        let clock = SimClock::new(Timestamp::from_nanos(12345));
        clock.advance(100);
        clock.tick(200);
        let saved = clock.state();
        let restored = SimClock::from_state(saved);
        assert_eq!(restored.now(), clock.now());
    }

    #[test]
    fn deterministic_same_tick_sequence() {
        let run = || {
            let clock = SimClock::at_epoch();
            let ticks = [100_u64, 200, 50, 1000, 1];
            let mut snapshots = Vec::new();
            for &t in &ticks {
                clock.tick(t);
                snapshots.push(clock.now());
            }
            snapshots
        };
        assert_eq!(run(), run());
    }

    // -----------------------------------------------------------------------
    // MemStorage tests
    // -----------------------------------------------------------------------

    #[test]
    fn storage_write_read_roundtrip() {
        let storage = MemStorage::new();
        let data = b"hello world";
        storage.write(0, data).unwrap();
        #[allow(clippy::cast_possible_truncation)]
        let read_back = storage.read(0, data.len() as u32).unwrap();
        assert_eq!(read_back, data);
    }

    #[test]
    fn storage_read_beyond_allocation_returns_zeros() {
        let storage = MemStorage::new();
        let result = storage.read(0, 16).unwrap();
        assert_eq!(result, vec![0u8; 16]);
    }

    #[test]
    fn storage_fsync_makes_durable() {
        let storage = MemStorage::new();
        storage.write(0, b"durable").unwrap();
        storage.fsync().unwrap();
        assert_eq!(storage.unflushed_count(), 0);
    }

    #[test]
    fn storage_crash_discards_unflushed() {
        let storage = MemStorage::new();
        storage.write(0, b"committed").unwrap();
        storage.fsync().unwrap();
        storage.write(0, b"ephemeral").unwrap();
        storage.crash();
        let result = storage.read(0, 9).unwrap();
        assert_eq!(result, b"committed");
    }

    #[test]
    fn storage_allocate() {
        let storage = MemStorage::new();
        let offset = storage.allocate(1024).unwrap();
        assert_eq!(offset, 0);
        let offset2 = storage.allocate(512).unwrap();
        assert_eq!(offset2, 1024);
    }

    #[test]
    fn storage_bytes_tracking() {
        let storage = MemStorage::new();
        storage.write(0, &[1, 2, 3]).unwrap();
        assert_eq!(storage.bytes_written(), 3);
        let _ = storage.read(0, 3).unwrap();
        assert_eq!(storage.bytes_read(), 3);
    }

    #[test]
    fn storage_with_data() {
        let initial = vec![1, 2, 3, 4, 5];
        let storage = MemStorage::with_data(initial.clone());
        let result = storage.read(0, 5).unwrap();
        assert_eq!(result, initial);
    }

    #[test]
    fn storage_recover_reverts_to_stable() {
        let storage = MemStorage::new();
        storage.write(0, b"stable").unwrap();
        storage.fsync().unwrap();
        storage.write(0, b"unstab").unwrap();
        storage.recover();
        let result = storage.read(0, 6).unwrap();
        assert_eq!(result, b"stable");
    }

    // -----------------------------------------------------------------------
    // MemNetwork tests
    // -----------------------------------------------------------------------

    #[test]
    fn network_send_recv_basic() {
        let net = MemNetwork::new();
        let r1 = ReplicaId(1);
        let r2 = ReplicaId(2);

        net.send(r1, r2, b"hello".to_vec()).unwrap();
        let (from, msg) = net.recv(r2).unwrap();
        assert_eq!(from, r1);
        assert_eq!(msg, b"hello");
    }

    #[test]
    fn network_recv_empty() {
        let net = MemNetwork::new();
        assert!(net.recv(ReplicaId(1)).is_none());
    }

    #[test]
    fn network_fifo_ordering() {
        let net = MemNetwork::new();
        let r1 = ReplicaId(1);
        let r2 = ReplicaId(2);

        net.send(r1, r2, b"first".to_vec()).unwrap();
        net.send(r1, r2, b"second".to_vec()).unwrap();

        let (_, msg1) = net.recv(r2).unwrap();
        let (_, msg2) = net.recv(r2).unwrap();
        assert_eq!(msg1, b"first");
        assert_eq!(msg2, b"second");
    }

    #[test]
    fn network_partition_blocks_messages() {
        let net = MemNetwork::new();
        let r1 = ReplicaId(1);
        let r2 = ReplicaId(2);

        net.partition(&[r1], &[r2]);
        let result = net.send(r1, r2, b"blocked".to_vec());
        assert_eq!(result, Err(NetworkError::Unreachable));
        assert!(net.recv(r2).is_none());
    }

    #[test]
    fn network_heal_restores_connectivity() {
        let net = MemNetwork::new();
        let r1 = ReplicaId(1);
        let r2 = ReplicaId(2);

        net.partition(&[r1], &[r2]);
        net.heal();
        net.send(r1, r2, b"healed".to_vec()).unwrap();
        let (_, msg) = net.recv(r2).unwrap();
        assert_eq!(msg, b"healed");
    }

    #[test]
    fn network_partition_bidirectional() {
        let net = MemNetwork::new();
        let r1 = ReplicaId(1);
        let r2 = ReplicaId(2);

        net.partition(&[r1], &[r2]);

        // Both directions blocked
        assert_eq!(
            net.send(r1, r2, b"a".to_vec()),
            Err(NetworkError::Unreachable)
        );
        assert_eq!(
            net.send(r2, r1, b"b".to_vec()),
            Err(NetworkError::Unreachable)
        );
    }

    #[test]
    fn network_counters() {
        let net = MemNetwork::new();
        let r1 = ReplicaId(1);
        let r2 = ReplicaId(2);

        net.send(r1, r2, b"msg1".to_vec()).unwrap();
        net.send(r1, r2, b"msg2".to_vec()).unwrap();
        assert_eq!(net.sent_count(), 2);

        let _ = net.recv(r2);
        assert_eq!(net.delivered_count(), 1);

        net.partition(&[r1], &[r2]);
        let _ = net.send(r1, r2, b"dropped".to_vec());
        assert_eq!(net.dropped_count(), 1);
    }

    #[test]
    fn replica_id_display() {
        assert_eq!(format!("{}", ReplicaId(5)), "replica-5");
    }

    // -----------------------------------------------------------------------
    // Send + Sync compile-time checks
    // -----------------------------------------------------------------------

    const _: () = {
        fn assert_send_sync<T: Send + Sync>() {}
        fn check() {
            assert_send_sync::<RealClock>();
            assert_send_sync::<SimClock>();
            assert_send_sync::<MemStorage>();
            assert_send_sync::<MemNetwork>();
        }
        let _ = check;
    };
}
