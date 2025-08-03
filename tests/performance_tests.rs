#[cfg(test)]
mod performance_tests {
    use super::*;
    use criterion::{criterion_group, criterion_main, Criterion};

    fn bench_small_input(c: &mut Criterion) {
        c.bench_function("small_input", |b| b.iter(|| perform_sync("tiny")));
    }

    fn bench_large_input(c: &mut Criterion) {
        let input = "a".repeat(10000);
        c.bench_function("large_input", |b| b.iter(|| perform_sync(&input)));
    }

    criterion_group!(benches, bench_small_input, bench_large_input);
    criterion_main!(benches);
}