#!/usr/bin/env python3
"""Local GitHub Releases API fixture for fail-closed publication tests."""

import argparse
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse


def expected_assets(version: str) -> list[str]:
    inputs = [
        f"lbc-{version}-x86_64-unknown-linux-gnu.tar.gz",
        f"lbc-{version}-x86_64-unknown-linux-gnu.tar.gz.sha256",
        f"lbc-{version}-universal-apple-darwin.tar.gz",
        f"lbc-{version}-universal-apple-darwin.tar.gz.sha256",
        f"lbc-{version}-x86_64-pc-windows-msvc.zip",
        f"lbc-{version}-x86_64-pc-windows-msvc.zip.sha256",
        f"lbc-{version}.cdx.json",
        f"lbc-{version}.provenance.json",
    ]
    return inputs + [f"{name}.asc" for name in inputs] + ["lbc-release-signing-key.asc"]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["success", "fail-upload", "public"], required=True)
    parser.add_argument("--port-file", type=Path, required=True)
    parser.add_argument("--state-file", type=Path, required=True)
    parser.add_argument("--version", required=True)
    args = parser.parse_args()
    expected = expected_assets(args.version)
    state = {
        "mode": args.mode,
        "created_draft": False,
        "draft": args.mode != "public",
        "published": args.mode == "public",
        "assets": [],
        "error": None,
    }

    def save_state() -> None:
        args.state_file.write_text(json.dumps(state, sort_keys=True), encoding="utf-8")

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, _format: str, *_values: object) -> None:
            return

        def send_json(self, status: int, value: object) -> None:
            encoded = json.dumps(value).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(encoded)))
            self.end_headers()
            self.wfile.write(encoded)

        def authorized(self) -> bool:
            if self.headers.get("Authorization") != "Bearer fixture-token":
                state["error"] = "missing fixture authorization"
                save_state()
                self.send_json(401, {"message": "unauthorized"})
                return False
            return True

        def do_GET(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
            if not self.authorized():
                return
            parsed = urlparse(self.path)
            if parsed.path.endswith(f"/releases/tags/v{args.version}"):
                if args.mode == "public":
                    self.send_json(
                        200,
                        {
                            "id": 1,
                            "draft": False,
                            "upload_url": f"http://127.0.0.1:{self.server.server_port}/uploads/1/assets{{?name,label}}",
                        },
                    )
                else:
                    self.send_json(404, {"message": "not found"})
                return
            if parsed.path.endswith("/releases/1/assets"):
                assets = [
                    {"id": index + 1, "name": name}
                    for index, name in enumerate(state["assets"])
                ]
                self.send_json(200, assets)
                return
            self.send_json(404, {"message": "unexpected GET"})

        def do_POST(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
            if not self.authorized():
                return
            parsed = urlparse(self.path)
            length = int(self.headers.get("Content-Length", "0"))
            body = self.rfile.read(length)
            if parsed.path.endswith("/releases"):
                payload = json.loads(body)
                if payload.get("draft") is not True:
                    state["error"] = "release was not created as a draft"
                    save_state()
                    self.send_json(422, {"message": state["error"]})
                    return
                state["created_draft"] = True
                state["draft"] = True
                save_state()
                self.send_json(
                    201,
                    {
                        "id": 1,
                        "draft": True,
                        "upload_url": f"http://127.0.0.1:{self.server.server_port}/uploads/1/assets{{?name,label}}",
                    },
                )
                return
            if parsed.path == "/uploads/1/assets":
                name = parse_qs(parsed.query).get("name", [""])[0]
                if name not in expected or name in state["assets"]:
                    state["error"] = f"unexpected upload {name!r}"
                    save_state()
                    self.send_json(422, {"message": state["error"]})
                    return
                if args.mode == "fail-upload" and len(state["assets"]) == 3:
                    save_state()
                    self.send_json(500, {"message": "injected upload failure"})
                    return
                state["assets"].append(name)
                save_state()
                self.send_json(201, {"id": len(state["assets"]), "name": name})
                return
            self.send_json(404, {"message": "unexpected POST"})

        def do_PATCH(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
            if not self.authorized():
                return
            parsed = urlparse(self.path)
            length = int(self.headers.get("Content-Length", "0"))
            payload = json.loads(self.rfile.read(length))
            if parsed.path.endswith("/releases/1") and payload == {"draft": False}:
                if sorted(state["assets"]) != sorted(expected):
                    state["error"] = "publication attempted with incomplete assets"
                    save_state()
                    self.send_json(422, {"message": state["error"]})
                    return
                state["draft"] = False
                state["published"] = True
                save_state()
                self.send_json(200, {"id": 1, "draft": False})
                return
            self.send_json(404, {"message": "unexpected PATCH"})

    save_state()
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    args.port_file.write_text(str(server.server_port), encoding="ascii")
    server.serve_forever()


if __name__ == "__main__":
    main()
