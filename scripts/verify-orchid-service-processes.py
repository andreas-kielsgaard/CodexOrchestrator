"""Isolated process smoke; no provider invocation or installed service changes."""
import json
import argparse
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time
import urllib.request
import uuid

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("binary_directory", type=Path, help="Directory containing orchid-host and orchid-server")
binary = parser.parse_args().binary_directory.resolve()
suffix = ".exe" if os.name == "nt" else ""
token = uuid.uuid4().hex
environment = dict(os.environ, ORCHID_SERVICE_TOKEN=token)
processes = []
logs = []


def request(url, body=None):
    payload = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(url, payload, {"Authorization": f"Bearer {token}", "Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=5) as response:
        return json.load(response)


def wait(check, label):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        value = check()
        if value:
            return value
        time.sleep(0.05)
    raise AssertionError(f"Timed out: {label}")


with tempfile.TemporaryDirectory(prefix="orchid-process-proof-") as temporary:
    root = Path(temporary)

    def launch(role, address, server=None):
        path = root / f"{role}-{len(processes)}.log"
        log = path.open("wb")
        logs.append(log)
        args = [str(binary / f"orchid-{role}{suffix}"), "--data-dir", str(root / role), "--listen", address]
        if server:
            args += ["--server", server]
        process = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT, env=environment)
        processes.append(process)

        def ready():
            text = path.read_text(errors="replace")
            if process.poll() is not None:
                raise AssertionError(f"{role} exited: {text}")
            match = re.search(r"listening on (127\.0\.0\.1:\d+)", text)
            return match.group(1) if match else None

        return process, wait(ready, role)

    try:
        router, address = launch("server", "127.0.0.1:0")
        server_url = f"http://{address}"
        host, local_address = launch("host", "127.0.0.1:0", server_url)
        device = request(f"http://{local_address}/v1/commands", {"method": "device.description", "input": {}})["id"]
        wait(lambda: device in request(f"{server_url}/v1/devices"), "initial host connection")
        assert request(f"{server_url}/v1/devices/{device}/commands", {"method": "device.description", "input": {}})["id"] == device
        router.terminate()
        router.wait(timeout=15)
        assert host.poll() is None
        assert request(f"http://{local_address}/v1/sessions") == []
        router, restarted_address = launch("server", address)
        assert restarted_address == address
        wait(lambda: device in request(f"{server_url}/v1/devices"), "host reconnection")
        assert request(f"{server_url}/v1/devices/{device}/commands", {"method": "device.description", "input": {}})["id"] == device
        assert host.poll() is None
        print(json.dumps({"separateProcesses": True, "hostSurvivedRouterRestart": True, "localApiAvailableWithoutRouter": True, "stableDeviceIdentity": True}))
    finally:
        for process in reversed(processes):
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        for log in logs:
            log.close()
