"""Small stdio MCP server used by the parallel-tool-calling investigation."""

import json
import os
import sys
import threading
import time


WRITE_LOCK = threading.Lock()
LOG_PATH = os.environ.get("ORCHID_PARALLEL_PROBE_LOG")
RUN = os.environ.get("ORCHID_PARALLEL_PROBE_RUN", "unspecified")


def write_json(value):
    with WRITE_LOCK:
        sys.stdout.write(json.dumps(value, separators=(",", ":")) + "\n")
        sys.stdout.flush()


def record(tool, phase):
    if not LOG_PATH:
        return
    event = {
        "run": RUN,
        "tool": tool,
        "phase": phase,
        "monotonicSeconds": time.perf_counter(),
    }
    with WRITE_LOCK:
        with open(LOG_PATH, "a", encoding="utf-8") as stream:
            stream.write(json.dumps(event, separators=(",", ":")) + "\n")


def tools():
    return [
        {
            "name": f"inspect_{label}",
            "description": f"Return the fixed {label} inspection marker after a short delay.",
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False},
            "annotations": {"readOnlyHint": True, "openWorldHint": False},
        }
        for label in ("alpha", "beta", "gamma", "delta")
    ]


def handle_call(request):
    request_id = request.get("id")
    name = request.get("params", {}).get("name", "")
    if name not in {tool["name"] for tool in tools()}:
        write_json(
            {
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {"code": -32601, "message": "Unknown probe tool"},
            }
        )
        return
    record(name, "started")
    time.sleep(1.5)
    record(name, "finished")
    write_json(
        {
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "content": [{"type": "text", "text": f"{name}=observed"}],
                "isError": False,
            },
        }
    )


def handle(request):
    method = request.get("method")
    if method == "initialize":
        write_json(
            {
                "jsonrpc": "2.0",
                "id": request.get("id"),
                "result": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "parallel-probe", "version": "1"},
                    "instructions": "Use the independent inspection tools requested by the caller.",
                },
            }
        )
    elif method == "tools/list":
        write_json(
            {
                "jsonrpc": "2.0",
                "id": request.get("id"),
                "result": {"tools": tools()},
            }
        )
    elif method == "tools/call":
        threading.Thread(target=handle_call, args=(request,), daemon=False).start()
    elif "id" in request:
        write_json({"jsonrpc": "2.0", "id": request["id"], "result": {}})


for line in sys.stdin:
    try:
        handle(json.loads(line))
    except Exception as error:
        write_json(
            {
                "jsonrpc": "2.0",
                "id": None,
                "error": {"code": -32603, "message": str(error)},
            }
        )
