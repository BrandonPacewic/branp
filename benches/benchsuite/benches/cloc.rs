use std::hint::black_box;

use benchsuite::fixtures;
use branp_cli::ops::cloc::count_paths_for_bench;
use criterion::{criterion_group, criterion_main, Criterion};

fn count_paths(c: &mut Criterion) {
    let fixtures = fixtures!();
    let mut group = c.benchmark_group("cloc/count_paths");
    group.sample_size(10);

    for input in fixtures.cloc_inputs() {
        group.bench_function(input.name, |b| {
            b.iter(|| {
                let report = count_paths_for_bench(std::slice::from_ref(black_box(&input.path)));
                black_box(report);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, count_paths);
criterion_main!(benches);
