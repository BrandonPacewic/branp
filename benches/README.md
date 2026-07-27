# bp Benchmarks

```sh
cd benches
cargo bench -p benchsuite --bench cloc
```

```sh
printf 'BP_CLOC_BENCH_REPO=/path/to/large/repo\n' > benchsuite/.env
cargo bench -p benchsuite --bench cloc -- cloc/count_paths/large-repo
```

```sh
cargo bench -p benchsuite --bench cloc -- --save-baseline main
cargo bench -p benchsuite --bench cloc -- --baseline main
```
