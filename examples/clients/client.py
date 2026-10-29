#!/usr/bin/env python3
"""Render the invoice example with urllib (standard library only).

Usage: TYPREST_URL=http://localhost:8080 python3 client.py [payload.json] [output.pdf]
"""
import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path

BASE_URL = os.environ.get("TYPREST_URL", "http://localhost:8080").rstrip("/")
DEFAULT_PAYLOAD = Path(__file__).resolve().parent.parent / "invoice/templates/invoice/payload.json"


def print_error(status: int, raw: bytes) -> None:
    print(f"request failed: HTTP {status}", file=sys.stderr)
    try:
        envelope = json.loads(raw)
        print(f"{envelope['error']}: {envelope['message']}", file=sys.stderr)
        for v in envelope.get("violations", []):
            print(f"  {v['field']}: {v['issue']}", file=sys.stderr)
        for d in envelope.get("diagnostics", []):
            loc = d.get("location")
            where = ""
            if loc:
                start = (loc.get("range") or {}).get("start")
                where = f" ({loc['path']}{':' + str(start['line']) if start else ''})"
            print(f"  [{d['severity']}] {d['message']}{where}", file=sys.stderr)
    except (ValueError, KeyError, TypeError):
        print(raw.decode("utf-8", "replace"), file=sys.stderr)


def main() -> int:
    payload_path = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_PAYLOAD
    output_path = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("invoice.pdf")
    request = urllib.request.Request(
        f"{BASE_URL}/invoice",
        data=payload_path.read_bytes(),
        headers={"content-type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request) as response:
            pdf = response.read()
    except urllib.error.HTTPError as err:
        print_error(err.code, err.read())
        return 1
    except urllib.error.URLError as err:
        print(f"request error: {err.reason}", file=sys.stderr)
        return 1
    output_path.write_bytes(pdf)
    print(f"saved {output_path} ({len(pdf)} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
