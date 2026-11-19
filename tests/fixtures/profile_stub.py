"""MCP fixture with matching call gates in fixed and over-listing modes."""
import json
import sys

core = ["core_tool"]
full = core + ["full_tool"]
profile = sys.argv[2] if len(sys.argv) == 3 else "core"
active = core if profile == "core" else full
expected_meta = {
    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
    "io.modelcontextprotocol/clientCapabilities": {},
}
methods = ["server/discover", "tools/list"]
for line in sys.stdin:
    request = json.loads(line)
    method = request["method"]
    assert request["params"]["_meta"] == expected_meta
    if methods:
        assert method == methods.pop(0)
    if method == "server/discover":
        result = {"_meta": {"io.modelcontextprotocol/serverInfo": {
            "name": "profile-stub", "version": "1",
        }}}
    elif method == "tools/list":
        names = full if sys.argv[1] == "buggy" else active
        result = {"tools": [{"name": name, "inputSchema": {"type": "object"}}
                            for name in names]}
    elif method == "tools/call":
        name = request["params"]["name"]
        result = {"isError": name not in active, "content": []}
    else:
        raise AssertionError(method)
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}),
          flush=True)
