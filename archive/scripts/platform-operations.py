#!/usr/bin/env python3
"""Manual Operations API client. Commands are read unchanged from a JSON file.

Keep the same command file when retrying after a timeout: its idempotency key
identifies the result across process restarts. Tokens never enter arguments.
"""
import argparse
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=["overview", "cases", "case", "audits", "costs", "command"])
    parser.add_argument("value", nargs="?", help="Case UUID, month YYYY-MM-01, or command JSON file")
    parser.add_argument("--after", help="Pagination cursor returned by the previous page")
    parser.add_argument("--case-id", help="Filter audits by case UUID")
    args = parser.parse_args()
    base = os.environ.get("NVBES_PLATFORM_OPERATIONS_URL", "http://127.0.0.1:8084").rstrip("/")
    url = urllib.parse.urlsplit(base)
    if url.scheme != "https" and not (url.scheme == "http" and url.hostname in ("localhost", "127.0.0.1")):
        parser.error("HTTPS required outside localhost")
    token = os.environ.get("NVBES_PLATFORM_OPERATIONS_ACCESS_TOKEN")
    if not token:
        parser.error("NVBES_PLATFORM_OPERATIONS_ACCESS_TOKEN required")
    path = "/api/v1/" + args.operation
    query = {}
    data = None
    if args.operation in ("case", "costs", "command") and not args.value:
        parser.error("value required for this operation")
    if args.operation == "case":
        import uuid
        path = "/api/v1/cases/" + str(uuid.UUID(args.value))
    elif args.operation == "costs":
        query["month"] = args.value
    elif args.operation == "command":
        path = "/api/v1/commands"
        cwd = os.path.abspath(os.getcwd())
        safe_path = os.path.abspath(os.path.join(cwd, args.value))
        rel = os.path.relpath(safe_path, cwd)
        if rel.startswith("..") or os.path.isabs(rel):
            parser.error("command file path must stay inside the current directory")
        with open(safe_path, encoding="utf-8") as command_file:
            data = json.dumps(json.load(command_file)).encode()
    if args.after:
        query["after"] = args.after
    if args.case_id:
        query["case_id"] = args.case_id
    if query:
        path += "?" + urllib.parse.urlencode(query)
    request = urllib.request.Request(base + path, data=data, headers={
        "Authorization": "Bearer " + token, "Content-Type": "application/json",
    })
    # Never forward the operator credential through an HTTP redirect.
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, newurl):
            return None
    opener = urllib.request.build_opener(NoRedirect)
    try:
        with opener.open(request, timeout=30) as response:
            print(json.dumps(json.load(response), indent=2, ensure_ascii=False))
    except urllib.error.HTTPError as error:
        print(f"HTTP {error.code}: {error.read(16384).decode(errors='replace')}", file=sys.stderr)
        return 1
    except (urllib.error.URLError, TimeoutError):
        print("Result unknown. Retry the same command file; do not generate a new key.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
