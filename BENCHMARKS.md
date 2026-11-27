# Benchmarks — assay

Measured 2026-11-27 on primo: 13th Gen Intel Core i7-13620H (16 threads), 62 GiB RAM, cargo 1.100.0, by `benches/hot_paths.rs` (hand-rolled on `std::time`; warmup 100). Primo was under extreme parallel load during this run (load average well above the 16 threads from concurrent fleet work), so absolute numbers are inflated — treat them as order-of-magnitude baselines, not lab numbers. Assay is a library crate (`publish = false` by design, unchanged); its core operation is the conformance check run: canonical measurement plus a live server probe.

## Workloads

- `counter_measure_64_tools` — `counter::measure` (canonical key-sorted serialization + byte count + cl100k_base token count) on a fixed 64-tool fixture (canonical size 13,047 bytes); 2,000 iterations.
- `determinism_twin_run_1/2` — the same 2,000-iteration pass run twice; canonical outputs asserted byte-identical.
- `probe_run_stub` — full `probe::run` (spawn, modern-era discover, tools/list) against the repo's `tests/fixtures/profile_stub.py`; 20 runs.

## Results (2026-11-27)

| workload | iters | total | per-op | ops/s |
|---|---|---|---|---|
| counter_measure_64_tools | 2,000 | 18,253.7 ms | 9,126.8 µs | 110 |
| determinism_twin_run_1 | 2,000 | 25,623.7 ms | 12,811.8 µs | 78 |
| determinism_twin_run_2 | 2,000 | 25,364.2 ms | 12,682.1 µs | 79 |
| probe_run_stub | 20 | 13,113.0 ms | 655.65 ms | 2 |

Determinism: twin-run canonical outputs byte-identical.

## Example

`examples/end_to_end.rs` — measures a 3-tool fixture (203 bytes, 49 tokens), probes the stub server, and asserts its launch profile. Run: `cargo run --example end_to_end`.
