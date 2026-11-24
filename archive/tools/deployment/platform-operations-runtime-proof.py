"""Synthetic proof against a real Operations daemon and a disposable database."""
import base64
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
import uuid


def b64(data):
    return base64.urlsafe_b64encode(data).rstrip(b"=")


def main():
    binary = str(Path("target/debug/nvbes-platform-operations").resolve())
    with tempfile.TemporaryDirectory(prefix="nvbes-ops-proof-") as directory:
        private = Path(directory) / "private.pem"
        public = Path(directory) / "public.pem"
        subprocess.run(["openssl", "genpkey", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:2048", "-out", str(private)], check=True, capture_output=True)
        subprocess.run(["openssl", "pkey", "-in", str(private), "-pubout", "-out", str(public)], check=True, capture_output=True)
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        environment = dict(os.environ, NVBES_ENVIRONMENT="testing",
            NVBES_PLATFORM_OPERATIONS_PORT=str(port),
            NVBES_PLATFORM_OPERATIONS_PUBLIC_KEY_PEM=public.read_text(),
            NVBES_PLATFORM_OPERATIONS_ISSUER="synthetic-operator-issuer",
            NVBES_PLATFORM_OPERATIONS_SERVICES="[]")
        subprocess.run([binary, "migrate"], env=environment, check=True)
        now = int(time.time())
        claims = {"sub": "synthetic-operator", "role": "platform_owner", "amr": ["totp"],
            "auth_time": now, "nbf": now-1, "exp": now+600,
            "iss": "synthetic-operator-issuer", "aud": "platform-operations"}
        signing_input = b64(b'{"alg":"RS256","typ":"JWT"}') + b"." + b64(json.dumps(claims).encode())
        signature = subprocess.run(["openssl", "dgst", "-sha256", "-sign", str(private)], input=signing_input, capture_output=True, check=True).stdout
        token = (signing_input + b"." + b64(signature)).decode()
        base = f"http://127.0.0.1:{port}"

        def request(path, data=None, auth=True):
            headers = {"Content-Type": "application/json"}
            if auth:
                headers["Authorization"] = "Bearer " + token
            req = urllib.request.Request(base + path, headers=headers,
                data=json.dumps(data).encode() if data is not None else None)
            try:
                with urllib.request.urlopen(req, timeout=20) as response:
                    body = response.read()
                    return response.status, json.loads(body) if body else None
            except urllib.error.HTTPError as error:
                return error.code, error.read().decode()

        def start():
            process = subprocess.Popen([binary], env=environment, stdout=subprocess.DEVNULL)
            for _ in range(100):
                if process.poll() is not None:
                    raise RuntimeError("proof daemon exited")
                try:
                    with urllib.request.urlopen(base + "/health/ready", timeout=1) as response:
                        if response.status == 200:
                            return process
                except (urllib.error.URLError, TimeoutError):
                    time.sleep(0.1)
            process.terminate()
            process.wait(timeout=10)
            raise RuntimeError("proof readiness timeout")

        def command(action):
            return {"idempotency_key": str(uuid.uuid4()), "correlation_id": str(uuid.uuid4()),
                "reason": "Synthetic lot 7 runtime proof", "action": action}

        process = start()
        try:
            assert request("/api/v1/overview", auth=False)[0] == 401
            code, overview = request("/api/v1/overview")
            assert code == 200 and overview["backup_restore"] == "not_verified"
            assert request("/api/v1/actions", {})[0] == 501
            create = command({"type": "open_case", "category": "support", "owner": "account",
                "subject_id": str(uuid.uuid4()), "source": "synthetic support ticket",
                "summary": "Delivery status question", "related_case_id": None})
            code, receipt = request("/api/v1/commands", create)
            assert code == 200, receipt
            case_id = receipt["case_id"]
            # Restart proves that the receipt does not depend on process memory.
            process.terminate()
            process.wait(timeout=10)
            process = start()
            assert request("/api/v1/commands", create)[1] == receipt
            conflicting = dict(create, reason="Changed reason with same idempotency key")
            assert request("/api/v1/commands", conflicting)[0] == 409
            spoofed = dict(create, operator_id="another-operator")
            assert request("/api/v1/commands", spoofed)[0] == 422
            version = 1
            observation = command({"type": "record_observation", "case_id": case_id, "expected_version": version,
                "service": "email", "observed_at": "2026-01-01T00:00:00Z",
                "api_reference": "synthetic email operations receipt", "summary": "Delivery verified by owner API"})
            assert request("/api/v1/commands", observation)[0] == 200
            version += 1
            for status in ["investigating", "awaiting_user", "investigating", "resolved", "closed", "appealed", "investigating", "resolved", "closed"]:
                update = command({"type":"transition", "case_id":case_id, "expected_version":version,
                    "status":status, "evidence":"Synthetic owner receipt and notification reference"})
                code, result = request("/api/v1/commands", update)
                assert code == 200, result
                version += 1
            code, context = request("/api/v1/cases/" + case_id)
            assert code == 200 and context["case"]["status"] == "closed"
            assert context["observations"][0]["recorded_by"] == "synthetic-operator"
            _, audits = request("/api/v1/audits?case_id=" + case_id)
            assert len(audits["items"]) == version
            assert all(item["receipt"]["actor"] == "synthetic-operator" for item in audits["items"])
            cost = command({"type":"record_cost", "month":"2026-09-01", "provider":"synthetic-provider",
                "category":"compute", "actual_cents":100, "forecast_cents":200,
                "evidence":"Synthetic invoice", "replaces":None})
            assert request("/api/v1/commands", cost)[0] == 200
            _, costs = request("/api/v1/costs?month=2026-09-01")
            assert costs["forecast_cents"] >= 200 and costs["tax_included"]
            print("PASS: signed operator auth, real HTTP lifecycle, restart replay, conflicts, evidence, appeal, audit and costs")
        finally:
            process.terminate()
            process.wait(timeout=10)


if __name__ == "__main__":
    main()
