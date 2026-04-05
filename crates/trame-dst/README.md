# trame-dst

Deterministic simulation testing (DST) and VOPR framework in Rust.

## What is DST?

Deterministic simulation testing replaces all sources of non-determinism (time, randomness, I/O) with simulated versions controlled by a single seed. Given the same seed, the simulation produces the identical execution every time. This lets you compress years of real-world operation into minutes and reproduce bugs that would otherwise take months to manifest in production.

## Quick Start

```rust
use trame_dst::prng::SplitMix64;

let mut rng = SplitMix64::new(42);

// Fork independent sub-generators for each component
let mut ops_rng = rng.fork();
let mut fault_rng = rng.fork();

// Both are deterministic: same seed always produces the same sequence
let value = ops_rng.next_u64();
let should_inject = fault_rng.chance(0.05);
```

## Features

- **SplitMix64 PRNG** -- Fast, statistically sound, splittable generator. Fork hierarchy ensures adding a new component does not change existing sequences.
- **32 fault types** across 5 categories (see table below).
- **Simulated I/O** -- Trait abstractions for Clock, Storage, and Network that run deterministically.
- **9-phase simulation loop** -- Generate ops, execute, inject faults, advance time, process deferred work, verify properties, check liveness, record metrics, report progress.
- **Failure shrinking** -- Automatically reduce a failing seed to the minimal reproduction.
- **CI tier configs** -- Commit, nightly, weekly, and pre-release profiles with tuned iteration counts and fault probabilities.
- **Coverage tracking** -- Variant coverage analysis to find untested fault/operation combinations.

## Fault Types

| Category | Count | Examples |
|----------|-------|---------|
| Storage | 10 | Write failure, partial write, bit flip, torn page, power loss, stale read |
| Network | 8 | Message drop, delay, reorder, duplicate, corrupt, full/asymmetric/partial partition |
| Process | 4 | Crash, restart, pause, slow |
| Clock | 3 | Skew, jump forward, stall |
| Composite | 7 | Power loss + restart, cascading failure, byzantine, split brain, rolling restart |

## Simulation Loop

```rust
use trame_dst::simulation::{Simulation, SimulationConfig, SimulationStatus};

let config = SimulationConfig {
    seed: 42,
    tick_count: 10_000,
    fault_probability: 0.05,
    ..Default::default()
};

let mut sim = Simulation::new(config);
// Provide your OperationGenerator, PropertyChecker, and deferred work callback
let result = sim.run(&mut my_generator, &my_checker, |_tick| {});

assert_eq!(result.status, SimulationStatus::Pass);
println!("Ticks: {}, Faults injected: {}", result.ticks_completed, result.faults_injected);
```

## Extension Points

Implement these traits to test your system:

- `OperationGenerator` -- Produces domain-specific operations each tick.
- `Operation` -- A single command executed against the system under test.
- `PropertyChecker` -- Verifies invariants hold after each batch of operations.
- `ReferenceModel` -- A simplified model to detect divergence from the real system.

## Inspired by

The deterministic simulation approach used in [TigerBeetle](https://tigerbeetle.com/) and [FoundationDB](https://www.foundationdb.org/).

## Part of trame

`trame-dst` is part of the [trame](https://github.com/copyleftdev/trame) workspace.

## License

Apache-2.0
