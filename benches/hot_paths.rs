//! Hot-path benchmarks for assay's core operation: `counter::measure` (canonicalize + byte/token count of a tools/list payload) on a fixed 64-tool fixture, a determinism twin-run, and `probe::run` against the repo's stub server. Hand-rolled on std::time; run with `cargo bench --bench hot_paths`.

use std::time::{Duration, Instant};

use assay::{counter, probe};
use serde_json::{json, Value};

/// Tools in the fixed measurement fixture (named bound).
const FIXTURE_TOOLS: usize = 64;
/// Warmup iterations before each measured loop.
const WARMUP: usize = 100;
/// Measured iterations for the core operation.
const MEASURED: usize = 2_000;
/// Measured probe runs against the stub server.
const PROBE_RUNS: usize = 20;
/// Per-response deadline for probe runs.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Build the fixed-size fixture: FIXTURE_TOOLS tools with a small
/// nested inputSchema each. Same value on every call.
fn fixture() -> Value {
    let tools: Vec<Value> = (0..FIXTURE_TOOLS)
        .map(|i| {
            json!({
                "name": format!("tool_{i:03}"),
                "description": format!("fixture tool number {i}"),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 0, "maximum": 100}
                    },
                    "required": ["path"]
                }
            })
        })
        .collect();
    Value::Array(tools)
}

fn report(name: &str, total: Duration, iterations: usize) {
    let per = total.as_secs_f64() / iterations as f64;
    println!(
        "{name}: {iterations} iters, total {:.1} ms, {:.1} us/op, {:.0} ops/s",
        total.as_secs_f64() * 1e3,
        per * 1e6,
        1.0 / per,
    );
}

fn main() {
    let tools = fixture();

    for _ in 0..WARMUP {
        let _ = counter::measure(&tools).expect("measure warmup");
    }
    let started = Instant::now();
    let mut bytes = 0usize;
    for _ in 0..MEASURED {
        let measured = counter::measure(&tools).expect("measure");
        bytes = measured.bytes;
    }
    report("counter_measure_64_tools", started.elapsed(), MEASURED);
    println!("counter_measure_64_tools: fixture canonical size = {bytes} bytes");

    // Determinism twin-run: two full passes must produce byte-identical
    // canonical output, and their timings are reported side by side.
    let mut canonicals: Vec<String> = Vec::new();
    for run in 0..2 {
        let started = Instant::now();
        let mut last = String::new();
        for _ in 0..MEASURED {
            last = counter::measure(&tools).expect("measure twin").canonical;
        }
        report(
            &format!("determinism_twin_run_{}", run + 1),
            started.elapsed(),
            MEASURED,
        );
        canonicals.push(last);
    }
    assert_eq!(
        canonicals[0], canonicals[1],
        "canonical output must be deterministic"
    );
    println!("determinism_twin_run: canonical outputs byte-identical");

    // Probe workload: full discover + tools/list against the stub.
    let stub = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/profile_stub.py"
    );
    let started = Instant::now();
    for _ in 0..PROBE_RUNS {
        let result =
            probe::run(&["python3", stub, "fixed"], &[], PROBE_TIMEOUT).expect("probe stub");
        assert_eq!(result.server_info["name"], "profile-stub");
    }
    report("probe_run_stub", started.elapsed(), PROBE_RUNS);
}
