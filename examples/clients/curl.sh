#!/usr/bin/env bash
# Render the invoice example with curl.
# Usage: TYPREST_URL=http://localhost:8080 ./curl.sh [payload.json] [output.pdf]
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
base_url="${TYPREST_URL:-http://localhost:8080}"
payload="${1:-$here/../invoice/templates/invoice/payload.json}"
output="${2:-invoice.pdf}"
body="$(mktemp)"
trap 'rm -f "$body"' EXIT

status="$(curl -sS -X POST "${base_url%/}/invoice" \
  -H 'content-type: application/json' \
  --data @"$payload" \
  -o "$body" -w '%{http_code}')"

if [ "$status" = "200" ]; then
  mv "$body" "$output"
  trap - EXIT
  echo "saved $output ($(wc -c < "$output") bytes)"
  exit 0
fi

# Error envelope: {"error": "...", "message": "...", "violations": [...]}
echo "request failed: HTTP $status" >&2
if command -v jq >/dev/null 2>&1; then
  jq -r '"\(.error): \(.message)",
         (.violations[]? | "  \(.field): \(.issue)"),
         (.diagnostics[]? | "  [\(.severity)] \(.message)")' "$body" >&2
else
  cat "$body" >&2; echo >&2
fi
exit 1
