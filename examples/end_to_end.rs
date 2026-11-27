//! Demonstrates assay end to end on a small fixture: canonicalize and measure a tools/list payload with `counter::measure`, then probe the repo's own stub MCP server with `probe::run` and check its launch profile with `launch_diff::assert_profiles`. Expected output: the fixture's byte/token counts, the stub's server name and tool names, and an OK line.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use assay::{counter, launch_diff, probe};
use serde_json::json;

/// Per-response deadline for the stub probe.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

fn run() -> Result<(), String> {
    // 1. The core operation: canonical measurement of a tools payload.
    let tools = json!([
        {"name": "alpha", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}}},
        {"name": "beta", "inputSchema": {"type": "object", "properties": {}}},
        {"name": "gamma", "inputSchema": {"type": "object"}}
    ]);
    let measured = counter::measure(&tools).map_err(|e| format!("measure fixture: {e}"))?;
    println!(
        "counter::measure: {} tools -> {} bytes, {} cl100k_base tokens",
        tools.as_array().map_or(0, Vec::len),
        measured.bytes,
        measured.tokens,
    );
    println!("canonical: {}", measured.canonical);

    // 2. Probe the fixture stub server (modern-era discover + tools/list).
    let stub = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/profile_stub.py"
    );
    let core = probe::run(&["python3", stub, "fixed"], &[], PROBE_TIMEOUT)
        .map_err(|e| format!("probe stub: {e}"))?;
    println!(
        "probe::run: server={} tools={}",
        core.server_info["name"].as_str().unwrap_or("?"),
        core.tools,
    );

    // 3. The launch-profile check the kit exists for.
    let profiles = BTreeMap::from([("core", core)]);
    let expected = BTreeMap::from([("core", BTreeSet::from(["core_tool".to_string()]))]);
    launch_diff::assert_profiles(&profiles, &expected)
        .map_err(|e| format!("assert_profiles: {e}"))?;
    println!("launch_diff::assert_profiles: core profile matches exactly");
    Ok(())
}

fn main() {
    match run() {
        Ok(()) => println!("end_to_end: OK"),
        Err(error) => {
            eprintln!("end_to_end: FAILED: {error}");
            std::process::exit(1);
        }
    }
}
