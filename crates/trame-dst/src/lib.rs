// Copyright 2026 Trame Contributors
// SPDX-License-Identifier: Apache-2.0

//! Deterministic simulation testing (DST) and VOPR framework for trame.
//!
//! This crate provides a domain-agnostic backbone for property-based,
//! fault-injecting, deterministic simulation testing. All randomness
//! derives from a single root seed via [`prng::SplitMix64`], enabling
//! perfect reproducibility: same seed, same execution, same outcome.
//!
//! # Architecture
//!
//! - [`prng`] — `SplitMix64` PRNG with fork hierarchy.
//! - [`fault`] — 32-variant fault injection engine.
//! - [`io`] — Trait abstractions for clock, storage, and network.
//! - [`simulation`] — 9-phase simulation loop.
//! - [`shrink`] — Failure case shrinking to minimal reproduction.
//! - [`ci`] — CI tier configuration (commit / nightly / weekly / pre-release).
//! - [`coverage`] — Variant coverage tracking and gap analysis.
//!
//! # Design Principles
//!
//! 1. **Total determinism** — No `std::time::SystemTime`, `thread_rng`, or OS
//!    entropy in the simulation path. All randomness flows from `SplitMix64`.
//! 2. **Single-threaded execution** — The simulation loop runs cooperatively
//!    on one thread, eliminating concurrency non-determinism.
//! 3. **Fork hierarchy** — Sub-PRNGs for each component (operations, faults,
//!    network, storage, scheduler, clock, data) are forked from the root,
//!    ensuring that adding a fault type does not change operation generation.
//! 4. **Domain-agnostic** — No domain-specific types. Consumers implement
//!    the `Operation`, `PropertyChecker`, and `ReferenceModel` traits.

#![forbid(unsafe_code)]

pub mod ci;
pub mod coverage;
pub mod fault;
pub mod io;
pub mod prng;
pub mod shrink;
pub mod simulation;
