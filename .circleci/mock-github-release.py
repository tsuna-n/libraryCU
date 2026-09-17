#!/usr/bin/env python3
"""Local GitHub Releases API fixture for fail-closed publication tests."""

import argparse
import hashlib
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--mode",
        choices=[
            "success",
            "resume",
            "fail-upload",
            "public",
            "api-error",
            "malformed",
            "unexpected",
            "duplicate",
            "tampered",
            "mutate-local",
            "public-race",
            "invalid-id",
        ],
        required=True,
    )
    parser.add_argument("--port-file", type=Path, required=True)
    parser.add_argument("--state-file", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--expected-assets-file", type=Path, required=True)
    parser.add_argument("--local-dist", type=Path)
    args = parser.parse_args()
    expected = args.expected_assets_file.read_text(encoding="utf-8").splitlines()
    if not expected or len(expected) != len(set(expected)):
        raise ValueError("expected asset manifest must be nonempty and unique")
    initial_assets: list[dict[str, object]] = []
    if args.mode == "invalid-id":
        initial_assets.append({"id": 0, "name": expected[0], "size": 1, "digest": "sha256:00", "state": "uploaded"})
    if args.mode == "resume":
        initial_assets.extend(
            {"id": index + 1, "name": name, "size": 1, "digest": "sha256:00", "state": "uploaded"}
            for index, name in enumerate(expected[:2])
        )
    if args.mode == "unexpected":
        initial_assets.append(
            {"id": 1, "name": "unexpected.txt", "size": 1, "digest": "sha256:00", "state": "uploaded"}
        )
    if args.mode == "duplicate":
        initial_assets.extend(
            {"id": index + 1, "name": expected[0], "size": 1, "digest": "sha256:00", "state": "uploaded"}
            for index in range(2)
        )
    state = {
        "mode": args.mode,
        "created_draft": False,
        "draft": args.mode != "public",
        "published": args.mode == "public",
        "assets": initial_assets,
        "uploads": [],
        "deletes": [],
        "error": None,
        "published_by_client": False,
        "publication_payload": None,
    }
    next_asset_id = max((int(asset["id"]) for asset in initial_assets), default=0) + 1

    def save_state() -> None:
        args.state_file.write_text(json.dumps(state, sort_keys=True), encoding="utf-8")

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, _format: str, *_values: object) -> None:
            return

        def send_json(self, status: int, value: object) -> None:
            if isinstance(value, dict) and "draft" in value:
                value = {"tag_name": f"v{args.version}", **value}
                if args.mode == "resume":
                    value = {"name": f"v{args.version} — DRAFT", "body": "DRAFT — release candidate",
                             "prerelease": True, **value}
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
            if parsed.path.endswith("/releases/1"):
                self.send_json(200, {"id": 1, "draft": state["draft"]})
                return
            if parsed.path.endswith(f"/releases/tags/v{args.version}"):
                if args.mode == "api-error":
                    self.send_json(500, {"message": "injected API failure"})
                    return
                if args.mode == "malformed":
                    self.send_json(200, {"draft": True, "id": 1})
                    return
                if args.mode == "public":
                    self.send_json(
                        200,
                        {
                            "id": 1,
                            "draft": False,
                            "upload_url": f"http://127.0.0.1:{self.server.server_port}/uploads/1/assets{{?name,label}}",
                        },
                    )
                elif args.mode in {"resume", "unexpected", "duplicate", "invalid-id"}:
                    self.send_json(
                        200,
                        {
                            "id": 1,
                            "draft": True,
                            "upload_url": f"http://127.0.0.1:{self.server.server_port}/uploads/1/assets{{?name,label}}",
                        },
                    )
                else:
                    self.send_json(404, {"message": "not found"})
                return
            if parsed.path.endswith("/releases/1/assets"):
                assets = list(state["assets"])
                if args.mode == "tampered" and assets:
                    assets[0] = {**assets[0], "digest": "sha256:" + "0" * 64}
                self.send_json(200, assets)
                return
            self.send_json(404, {"message": "unexpected GET"})

        def do_POST(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
            nonlocal next_asset_id
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
                names = [asset["name"] for asset in state["assets"]]
                if name not in expected or name in names:
                    state["error"] = f"unexpected upload {name!r}"
                    save_state()
                    self.send_json(422, {"message": state["error"]})
                    return
                if args.mode == "fail-upload" and len(state["assets"]) == 3:
                    save_state()
                    self.send_json(500, {"message": "injected upload failure"})
                    return
                state["assets"].append(
                    {
                        "id": next_asset_id,
                        "name": name,
                        "size": len(body),
                        "digest": f"sha256:{hashlib.sha256(body).hexdigest()}",
                        "state": "uploaded",
                    }
                )
                next_asset_id += 1
                state["uploads"].append(name)
                if len(state["uploads"]) == 1:
                    if args.mode == "mutate-local" and args.local_dist:
                        with (args.local_dist / expected[0]).open("ab") as original:
                            original.write(b"injected original-directory mutation")
                    if args.mode == "public-race":
                        state["draft"] = False
                        state["published"] = True
                save_state()
                self.send_json(201, state["assets"][-1])
                return
            self.send_json(404, {"message": "unexpected POST"})

        def do_PATCH(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
            if not self.authorized():
                return
            parsed = urlparse(self.path)
            length = int(self.headers.get("Content-Length", "0"))
            payload = json.loads(self.rfile.read(length))
            if parsed.path.endswith("/releases/1") and payload.get("draft") is False:
                if (payload.get("prerelease") is not False or payload.get("name") != f"v{args.version}"
                        or not isinstance(payload.get("body"), str) or "DRAFT" in payload["body"]):
                    state["error"] = "final publication retained inconsistent candidate metadata"
                    save_state()
                    self.send_json(422, {"message": state["error"]})
                    return
                if sorted(asset["name"] for asset in state["assets"]) != sorted(expected):
                    state["error"] = "publication attempted with incomplete assets"
                    save_state()
                    self.send_json(422, {"message": state["error"]})
                    return
                state["draft"] = False
                state["published"] = True
                state["published_by_client"] = True
                state["publication_payload"] = payload
                save_state()
                self.send_json(200, {"id": 1, **payload})
                return
            self.send_json(404, {"message": "unexpected PATCH"})

        def do_DELETE(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
            if not self.authorized():
                return
            parsed = urlparse(self.path)
            if "/releases/assets/" not in parsed.path:
                self.send_json(404, {"message": "unexpected DELETE"})
                return
            asset_id = int(parsed.path.rsplit("/", 1)[-1])
            state["deletes"].append(asset_id)
            before = len(state["assets"])
            state["assets"] = [asset for asset in state["assets"] if asset["id"] != asset_id]
            if len(state["assets"]) == before:
                self.send_json(404, {"message": "asset not found"})
                return
            save_state()
            self.send_response(204)
            self.end_headers()

    save_state()
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    args.port_file.write_text(str(server.server_port), encoding="ascii")
    server.serve_forever()


if __name__ == "__main__":
    main()
