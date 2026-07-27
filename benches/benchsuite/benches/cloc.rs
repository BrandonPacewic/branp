use std::hint::black_box;
use std::process::Command;
use std::process::Stdio;
use std::time::{Duration, Instant};

use benchsuite::fixtures;
use benchsuite::ClocInput;
use branp_cli::ops::cloc::{count_paths_for_bench, BenchCountReport};
use criterion::{criterion_group, criterion_main, Criterion};

fn count_paths(c: &mut Criterion) {
    let fixtures = fixtures!();
    let inputs = fixtures.cloc_inputs();

    if std::env::var_os("BP_CLOC_BENCH_PROBE").is_some() {
        probe_one_shot_timings(&filtered_probe_inputs(&inputs));
    }

    let mut group = c.benchmark_group("cloc/count_paths");
    group.sample_size(10);

    for input in inputs {
        group.bench_function(input.name, |b| {
            b.iter(|| {
                let report = count_paths_for_bench(std::slice::from_ref(black_box(&input.path)));
                black_box(report);
            });
        });
    }

    group.finish();
}

fn probe_one_shot_timings(inputs: &[&ClocInput]) {
    for input in inputs {
        let (first_elapsed, first_report) = time_count(input);
        let (second_elapsed, second_report) = time_count(input);
        let (third_elapsed, third_report) = time_count(input);

        assert_eq!(first_report, second_report);
        assert_eq!(first_report, third_report);

        eprintln!(
            "cloc bench probe/{name}: first={first_elapsed:?}, second={second_elapsed:?}, third={third_elapsed:?}, files={files}, blank={blank}, comment={comment}, code={code}",
            name = input.name,
            files = first_report.text_files,
            blank = first_report.blank,
            comment = first_report.comment,
            code = first_report.code,
        );

        if let Some(bp) = std::env::var_os("BP_CLOC_BENCH_CLI") {
            let first_cli_elapsed = time_cli_count(&bp, input);
            let second_cli_elapsed = time_cli_count(&bp, input);
            eprintln!(
                "cloc bench probe-cli/{name}: first={first_cli_elapsed:?}, second={second_cli_elapsed:?}, binary={binary}",
                name = input.name,
                binary = bp.to_string_lossy(),
            );
        }
    }
}

fn filtered_probe_inputs(inputs: &[ClocInput]) -> Vec<&ClocInput> {
    let filters = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with('-'))
        .collect::<Vec<_>>();

    if filters.is_empty() {
        return inputs.iter().collect();
    }

    inputs
        .iter()
        .filter(|input| {
            filters
                .iter()
                .any(|filter| filter.contains(&input.name) || input.name.contains(filter))
        })
        .collect()
}

fn time_count(input: &ClocInput) -> (Duration, BenchCountReport) {
    let start = Instant::now();
    let report = count_paths_for_bench(std::slice::from_ref(&input.path));
    (start.elapsed(), report)
}

fn time_cli_count(binary: &std::ffi::OsStr, input: &ClocInput) -> Duration {
    let start = Instant::now();
    let status = Command::new(binary)
        .arg("cloc")
        .arg("--no-live")
        .arg("--no-commas")
        .arg(&input.path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.to_string_lossy()));
    assert!(status.success());
    start.elapsed()
}

criterion_group!(benches, count_paths);
criterion_main!(benches);
