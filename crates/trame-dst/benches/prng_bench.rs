// Copyright 2026 Trame Contributors
// SPDX-License-Identifier: Apache-2.0

//! Criterion benchmarks for the `SplitMix64` PRNG.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use trame_dst::prng::{SplitMix64, fork_components};

fn bench_next_u64(c: &mut Criterion) {
    c.bench_function("splitmix64_next_u64", |b| {
        let mut rng = SplitMix64::new(42);
        b.iter(|| black_box(rng.next_u64()));
    });
}

fn bench_next_f64(c: &mut Criterion) {
    c.bench_function("splitmix64_next_f64", |b| {
        let mut rng = SplitMix64::new(42);
        b.iter(|| black_box(rng.next_f64()));
    });
}

fn bench_fork(c: &mut Criterion) {
    c.bench_function("splitmix64_fork", |b| {
        let mut rng = SplitMix64::new(42);
        b.iter(|| black_box(rng.fork()));
    });
}

fn bench_fork_components(c: &mut Criterion) {
    c.bench_function("splitmix64_fork_components", |b| {
        b.iter(|| {
            let mut root = SplitMix64::new(42);
            black_box(fork_components(&mut root))
        });
    });
}

fn bench_chance(c: &mut Criterion) {
    c.bench_function("splitmix64_chance", |b| {
        let mut rng = SplitMix64::new(42);
        b.iter(|| black_box(rng.chance(0.5)));
    });
}

fn bench_range(c: &mut Criterion) {
    c.bench_function("splitmix64_range", |b| {
        let mut rng = SplitMix64::new(42);
        b.iter(|| black_box(rng.range(0, 1000)));
    });
}

fn bench_weighted_index(c: &mut Criterion) {
    c.bench_function("splitmix64_weighted_index_5", |b| {
        let mut rng = SplitMix64::new(42);
        let weights = [10, 30, 20, 25, 15];
        b.iter(|| black_box(rng.weighted_index(&weights)));
    });
}

fn bench_shuffle_100(c: &mut Criterion) {
    c.bench_function("splitmix64_shuffle_100", |b| {
        let mut rng = SplitMix64::new(42);
        let mut data: Vec<u32> = (0..100).collect();
        b.iter(|| {
            rng.shuffle(&mut data);
            black_box(&data);
        });
    });
}

fn bench_next_uuid(c: &mut Criterion) {
    c.bench_function("splitmix64_next_uuid", |b| {
        let mut rng = SplitMix64::new(42);
        b.iter(|| black_box(rng.next_uuid()));
    });
}

fn bench_choose(c: &mut Criterion) {
    c.bench_function("splitmix64_choose_100", |b| {
        let mut rng = SplitMix64::new(42);
        let items: Vec<u32> = (0..100).collect();
        b.iter(|| black_box(rng.choose(&items)));
    });
}

criterion_group!(
    benches,
    bench_next_u64,
    bench_next_f64,
    bench_fork,
    bench_fork_components,
    bench_chance,
    bench_range,
    bench_weighted_index,
    bench_shuffle_100,
    bench_next_uuid,
    bench_choose,
);
criterion_main!(benches);
