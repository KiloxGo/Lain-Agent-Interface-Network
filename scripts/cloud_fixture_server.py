#!/usr/bin/env python3
"""Temporary Linux acceptance fixture, never an application service.

Runs the real Kernel with a separate synthetic DB, a bounded local Responses
fixture and an authenticated fault-control gateway. Provisioning/HTTPS routing
and teardown belong to the private operator manifest, not this module.
"""
import argparse
import contextlib
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import http.client
import json
import os
from pathlib import Path
import secrets
import signal
import socket
import subprocess
import threading
import time
import urllib.error
import urllib.request
import uuid


class Fixture:
    def __init__(self, directory):
        self.directory = directory
        self.config = json.loads((directory / "fixture.json").read_text())
        self.stop = threading.Event()
        self.lock = threading.Lock()
        self.kernel_lock = threading.Lock()
        self.kernel = None
        self.kernel_binary_sha256 = None
        self.blocked = False
        self.dispatch = None
        self.memory_probe = None
        self.drop_kind = None
        self.drop_remaining = 0
        self.dropped = []
        self.requests = []
        self.active = set()
        self.errors = []
        self.servers = []
        self.threads = []

    def disconnect(self):
        with self.lock:
            connections = list(self.active)
        for upstream, downstream in connections:
            for connection in (upstream.sock, downstream):
                if connection:
                    with contextlib.suppress(OSError):
                        connection.shutdown(socket.SHUT_RDWR)

    def start_kernel(self):
        environment = {k: v for k, v in os.environ.items()
                       if not k.upper().startswith(("LAIN_", "OPENAI_", "DEEPSEEK_", "ANTHROPIC_"))
                       and k.upper() not in {"HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY"}}
        environment.update(
            LAIN_LISTEN="127.0.0.1:" + str(self.config["kernel_port"]),
            LAIN_DATABASE_PATH=str(self.directory / "kernel.sqlite3"),
            LAIN_API_TOKEN=self.config["owner_token"],
            LAIN_SECRET_ACTOR=self.config["model_token"],
            LAIN_ACTOR_CONFIG=str(self.directory / "actor.json"),
            LAIN_CODEX_PATH=str(self.directory / "no-cloud-worker"),
            LAIN_LOG="error",
        )
        log = (self.directory / "kernel.log").open("ab")
        try:
            self.kernel = subprocess.Popen([self.config["kernel_binary"]], cwd=self.directory,
                                           env=environment, stdin=subprocess.DEVNULL,
                                           stdout=log, stderr=log)
        finally:
            log.close()
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        until = time.monotonic() + 15
        while not self.stop.is_set():
            if self.kernel.poll() is not None:
                raise RuntimeError("isolated Kernel exited during startup")
            try:
                request = urllib.request.Request(
                    "http://127.0.0.1:" + str(self.config["kernel_port"]) + "/ready",
                    headers={"Authorization": "Bearer " + self.config["owner_token"]})
                with opener.open(request, timeout=1) as response:
                    if response.status == 200:
                        self.kernel_binary_sha256 = hashlib.sha256(
                            Path('/proc/' + str(self.kernel.pid) + '/exe').read_bytes()).hexdigest()
                        expected = self.config.get("kernel_binary_sha256")
                        if expected and self.kernel_binary_sha256 != expected:
                            raise ValueError("running fixture Kernel differs from selected candidate")
                        return
            except (OSError, urllib.error.URLError):
                pass
            if time.monotonic() >= until:
                raise RuntimeError("isolated Kernel readiness timeout")
            self.stop.wait(.05)
        raise RuntimeError("fixture stopped during startup")

    def stop_kernel(self):
        if self.kernel and self.kernel.poll() is None:
            self.kernel.terminate()
            try:
                self.kernel.wait(timeout=20)
            except subprocess.TimeoutExpired:
                self.kernel.kill()
                self.kernel.wait(timeout=5)

    def control(self, command):
        action = command.get("action")
        if action == "dispatch":
            with self.lock:
                self.dispatch = command["input"]
        elif action == "memory_probe":
            prompt, steps = command.get("prompt"), command.get("steps")
            if (not isinstance(prompt, str) or not prompt.startswith("cloud memory probe ")
                    or len(prompt) > 128 or not isinstance(steps, list) or not 1 <= len(steps) <= 2
                    or len(json.dumps(steps).encode()) > 16384
                    or any(not isinstance(step, dict) or set(step) != {"name", "arguments"}
                           or step["name"] not in ("memory_inspect", "memory_search", "memory_propose")
                           or not isinstance(step["arguments"], dict) for step in steps)):
                raise ValueError("invalid bounded memory probe")
            with self.lock:
                self.memory_probe = {"prompt": prompt, "steps": steps}
        elif action == "block":
            with self.lock:
                self.blocked = bool(command["value"])
            if self.blocked:
                self.disconnect()
        elif action == "lose_receipt":
            with self.lock:
                self.drop_kind = command["kind"]
                self.drop_remaining = 1
        elif action == "restart_kernel":
            with self.kernel_lock:
                self.disconnect()
                self.stop_kernel()
                self.start_kernel()
        elif action == "stats":
            pass
        elif action == "model_inputs":
            with self.lock:
                return {"requests": list(self.requests)}
        else:
            raise ValueError("unsupported fixture control")
        with self.lock:
            return {"model_calls": len(self.requests), "blocked": self.blocked,
                    "dropped_receipts": list(self.dropped), "errors": list(self.errors),
                    "kernel_pid": self.kernel.pid, "kernel_running": self.kernel.poll() is None,
                    "kernel_binary_sha256": self.kernel_binary_sha256}

    def respond(self, body):
        encoded = json.dumps(body)
        if any(secret in encoded for secret in (self.config["owner_token"], self.config["model_token"])):
            raise ValueError("credential entered model input")
        if body.get("store") is not False or body.get("stream") is not True:
            raise ValueError("unexpected model request mode")
        with self.lock:
            if len(self.requests) >= 64:
                raise ValueError("fixture model call budget exhausted")
            self.requests.append(body)
            dispatch = self.dispatch.copy() if self.dispatch else None
            probe = self.memory_probe.copy() if self.memory_probe else None
        items = body["input"]
        # Only exact synthetic commands drive this fixture. Kernel-attributed
        # evidence also uses user-role frames; it is data, never a test command.
        last_user = next(index for index in range(len(items) - 1, -1, -1)
                         if items[index].get("role") == "user"
                         and isinstance(items[index].get("content"), str)
                         and (items[index]["content"] == "cloud-delegate"
                              or items[index]["content"].startswith("cloud parallel chat")
                              or items[index]["content"].startswith("cloud memory probe ")))
        prompt = items[last_user]["content"]
        outputs = [item for item in items[last_user + 1:] if item.get("type") == "function_call_output"]
        if prompt.startswith("cloud memory probe "):
            if not probe or probe["prompt"] != prompt:
                raise ValueError("memory probe not configured for this exact request")
            if len(outputs) < len(probe["steps"]):
                step = probe["steps"][len(outputs)]
                return [{"type": "function_call", "name": step["name"], "call_id": str(uuid.uuid4()),
                         "arguments": json.dumps(step["arguments"])}]
            # The expected source marker is never put in the reply by the
            # controller: only actual current Kernel tool output can supply it.
            text = "fixture-memory-reply " + str(outputs[-1].get("output", ""))
        elif outputs:
            text = "fixture acknowledged the recorded result"
        else:
            if prompt == "cloud-delegate":
                if not dispatch:
                    raise ValueError("dispatch fixture not configured")
                return [{"type": "function_call", "name": "device_queue", "call_id": str(uuid.uuid4()),
                         "arguments": json.dumps(dispatch)}]
            text = "fixture reply: " + prompt
        return [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": text}]}]

    def serve(self):
        fixture = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def setup(self):
                self.request.settimeout(45)
                super().setup()

            def reply(self, code, value):
                payload = json.dumps(value).encode()
                self.send_response(code)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)
                self.wfile.flush()

            def body(self):
                length = int(self.headers.get("Content-Length", "0"))
                if not 0 <= length <= 1024 * 1024:
                    raise ValueError("fixture request exceeds budget")
                return self.rfile.read(length)

        class Model(Handler):
            def do_POST(self):
                try:
                    if self.path != "/v1/responses" or not secrets.compare_digest(
                            self.headers.get("Authorization", ""), "Bearer " + fixture.config["model_token"]):
                        self.reply(401, {"error": "fixture authentication required"})
                        return
                    output = fixture.respond(json.loads(self.body()))
                    wire = b"data: " + json.dumps({"type": "response.completed", "response": {
                        "status": "completed", "output": output, "usage": {"total_tokens": 10}}}).encode() + b"\n\n"
                    self.send_response(200)
                    self.send_header("Content-Type", "text/event-stream")
                    self.send_header("Content-Length", str(len(wire)))
                    self.end_headers()
                    self.wfile.write(wire)
                    self.wfile.flush()
                except (ConnectionError, OSError):
                    pass
                except Exception as error:
                    with fixture.lock:
                        fixture.errors.append(type(error).__name__)
                    self.reply(503, {"error": "fixture rejected model input"})

        class Gateway(Handler):
            def do_GET(self):
                self.forward()

            def do_POST(self):
                self.forward()

            def forward(self):
                started = False
                connection = None
                try:
                    body = self.body()
                    if self.path == "/__fixture/control":
                        if self.command != "POST" or not secrets.compare_digest(
                                self.headers.get("Authorization", ""), "Bearer " + fixture.config["owner_token"]):
                            self.reply(401, {"error": "fixture owner required"})
                        else:
                            self.reply(200, fixture.control(json.loads(body)))
                        return
                    with fixture.lock:
                        blocked = fixture.blocked
                    if blocked:
                        self.reply(503, {"error": "synthetic connection loss"})
                        return
                    connection = http.client.HTTPConnection("127.0.0.1", fixture.config["kernel_port"], timeout=3)
                    headers = {k: v for k, v in self.headers.items()
                               if k.lower() not in {"host", "connection", "transfer-encoding", "content-length"}}
                    headers["Content-Length"] = str(len(body))
                    with fixture.lock:
                        admitted = not fixture.blocked
                        if admitted:
                            fixture.active.add((connection, self.connection))
                            # Publish an actual socket before block can capture
                            # the active set; later send cannot silently connect
                            # a fresh socket after that partition boundary.
                            connection.connect()
                            connection.sock.settimeout(40)
                    if not admitted:
                        self.reply(503, {"error": "synthetic connection loss"})
                        return
                    connection.request(self.command, self.path, body, headers)
                    response = connection.getresponse()
                    if self.command == "POST" and self.path.endswith("/events") and response.status < 300:
                        event = json.loads(body)
                        with fixture.lock:
                            drop = fixture.drop_remaining > 0 and event.get("kind") == fixture.drop_kind
                            if drop:
                                fixture.drop_remaining -= 1
                                fixture.dropped.append({"kind": event["kind"], "sequence": event["sequence"]})
                        if drop:
                            response.read(1024 * 1024)
                            self.connection.shutdown(socket.SHUT_RDWR)
                            self.close_connection = True
                            return
                    self.send_response(response.status)
                    for key in ("Content-Type", "Content-Length"):
                        if response.getheader(key):
                            self.send_header(key, response.getheader(key))
                    self.send_header("Connection", "close")
                    self.end_headers()
                    started = True
                    total = 0
                    while not fixture.stop.is_set():
                        chunk = response.read1(65536)
                        if not chunk:
                            break
                        total += len(chunk)
                        if total > 32 * 1024 * 1024:
                            raise ValueError("fixture response exceeds budget")
                        self.wfile.write(chunk)
                        self.wfile.flush()
                    self.close_connection = True
                except (ConnectionError, OSError, http.client.HTTPException):
                    if not started:
                        with contextlib.suppress(OSError):
                            self.reply(503, {"error": "fixture upstream disconnected"})
                except Exception as error:
                    with fixture.lock:
                        fixture.errors.append(type(error).__name__)
                    if not started:
                        self.reply(400, {"error": "fixture rejected request"})
                finally:
                    if connection:
                        with fixture.lock:
                            fixture.active.discard((connection, self.connection))
                        connection.close()

        model = ThreadingHTTPServer(("127.0.0.1", 0), Model)
        gateway = ThreadingHTTPServer(("127.0.0.1", self.config["gateway_port"]), Gateway)
        for server in (model, gateway):
            server.daemon_threads = True
            self.servers.append(server)
        actor = {"driver": "openai-responses", "model": "fixture-cloud-acceptance",
                 "endpoint": "http://127.0.0.1:" + str(model.server_port) + "/v1",
                 "secret_ref": "secret://env/actor", "timeout_seconds": 30,
                 "max_output_tokens": 1024, "max_turn_tokens": 4096}
        (self.directory / "actor.json").write_text(json.dumps(actor))
        (self.directory / "actor.json").chmod(0o600)
        try:
            for server in self.servers:
                thread = threading.Thread(target=server.serve_forever, daemon=True)
                self.threads.append(thread)
                thread.start()
            self.start_kernel()
            (self.directory / "status.json").write_text(json.dumps({"phase": "ready", "kernel_pid": self.kernel.pid,
                "gateway_port": gateway.server_port, "model_port": model.server_port}))
            self.stop.wait()
        finally:
            self.stop.set()
            self.disconnect()
            with self.kernel_lock:
                self.stop_kernel()
            for server in self.servers:
                server.shutdown()
                server.server_close()
            for thread in self.threads:
                thread.join(timeout=5)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    fixture = Fixture(args.directory.resolve(strict=True))
    signal.signal(signal.SIGTERM, lambda *_: fixture.stop.set())
    signal.signal(signal.SIGINT, lambda *_: fixture.stop.set())
    fixture.serve()


if __name__ == "__main__":
    main()
