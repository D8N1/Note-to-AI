// Minimal performance benchmark for note-to-ai
#![allow(unused)]
use criterion::{criterion_group, criterion_main, Criterion};

fn dummy_benchmark(_c: &mut Criterion) {
    // Add real benchmarks here
}

criterion_group!(benches, dummy_benchmark);
criterion_main!(benches);
