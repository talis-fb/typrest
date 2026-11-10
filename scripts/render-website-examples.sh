#!/usr/bin/env bash
# Render the website's example previews and Web UI screenshot.
#
# Usage: scripts/render-website-examples.sh
#
# For every example project with a templates/**/payload.json, starts `typrest serve`, waits on
# /_healthz, posts each payload, and writes:
#   website/public/examples/<example>/<template>.pdf       the PDF, served as is
#   website/src/assets/examples/<example>/<template>.png   a first-page preview, optimized by Astro
#   website/src/assets/examples/manifest.json             what the gallery reads
# Then, when Chrome or Chromium is available, captures website/src/assets/webui.png from the
# invoice project.
#
# The output is committed, so building the site needs no Rust toolchain. Run this by hand when
# templates change. `packages` and `ticket` download Typst packages on their first run.
#
# Requires: curl, pdftoppm and pdfinfo (poppler), python3, and node for the screenshot.
# Environment: TYPREST_BIN (default target/release/typrest, built if missing), PORT (default
# 18080), CHROME (default: first of google-chrome, chromium, chromium-browser on PATH).
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
bin="${TYPREST_BIN:-$root/target/release/typrest}"
port="${PORT:-18080}"
base_url="http://127.0.0.1:$port"
pdf_dir="$root/website/public/examples"
img_dir="$root/website/src/assets/examples"
work="$(mktemp -d)"
server_pid=""

cleanup() {
  stop_server
  rm -rf "$work"
}

stop_server() {
  if [ -n "$server_pid" ]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
    server_pid=""
  fi
}

trap cleanup EXIT

for tool in curl pdftoppm pdfinfo python3; do
  command -v "$tool" >/dev/null || { echo "error: $tool is required" >&2; exit 1; }
done

if [ ! -x "$bin" ]; then
  echo "Building typrest..."
  cargo build --release --locked --bin typrest --manifest-path "$root/Cargo.toml"
fi

# start_server <project-dir>: serves the project and waits until /_healthz answers. Startup
# compiles every template (and may download packages), so allow a generous timeout.
start_server() {
  local project="$1" log="$work/$(basename "$1").log"
  (cd "$project" && exec "$bin" serve --hostname 127.0.0.1 --port "$port") >"$log" 2>&1 &
  server_pid=$!
  for _ in $(seq 1 300); do
    if curl -fsS "$base_url/_healthz" >/dev/null 2>&1; then
      return 0
    fi
    if ! kill -0 "$server_pid" 2>/dev/null; then
      echo "error: typrest exited while starting $project:" >&2
      cat "$log" >&2
      exit 1
    fi
    sleep 0.5
  done
  echo "error: $project did not become healthy; log:" >&2
  cat "$log" >&2
  exit 1
}

# templates_root <project-dir>: the templates_root value from typrest.toml.
templates_root() {
  sed -nE 's/^templates_root[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$1/typrest.toml"
}

rm -rf "$pdf_dir" "$img_dir"
mkdir -p "$pdf_dir" "$img_dir"
rows="$work/manifest.tsv"
: >"$rows"

for project in "$root"/examples/*/; do
  project="${project%/}"
  name="$(basename "$project")"
  if [ ! -f "$project/typrest.toml" ]; then
    echo "Skipping $name: no typrest.toml"
    continue
  fi
  templates="$project/$(templates_root "$project")"
  mapfile -t payloads < <(find "$templates" -name payload.json -not -path '*/_*' | sort)
  if [ "${#payloads[@]}" -eq 0 ]; then
    echo "Skipping $name: no payload.json"
    continue
  fi

  echo "Rendering $name..."
  start_server "$project"
  mkdir -p "$pdf_dir/$name" "$img_dir/$name"
  for payload in "${payloads[@]}"; do
    template="$(dirname "${payload#"$templates"/}")"
    slug="${template//\//-}"
    pdf="$pdf_dir/$name/$slug.pdf"
    curl -fsS -X POST "$base_url/$template" \
      -H 'Content-Type: application/json' \
      --data-binary "@$payload" \
      --output "$pdf"
    pdftoppm -png -singlefile -f 1 -l 1 -scale-to 1200 "$pdf" "$img_dir/$name/$slug"
    pages="$(pdfinfo "$pdf" | awk '/^Pages:/ { print $2 }')"
    printf '%s\t%s\t%s\t%s\n' "$name" "$template" "$slug" "$pages" >>"$rows"
    echo "  $template ($pages page(s))"
  done
  stop_server
done

python3 - "$rows" "$img_dir/manifest.json" <<'PY'
import json, sys

manifest = {}
with open(sys.argv[1], encoding="utf-8") as rows:
    for line in rows:
        name, template, slug, pages = line.rstrip("\n").split("\t")
        manifest.setdefault(name, []).append({
            "template": template,
            "pdf": f"{name}/{slug}.pdf",
            "image": f"{name}/{slug}.png",
            "pages": int(pages),
        })
with open(sys.argv[2], "w", encoding="utf-8") as out:
    json.dump(manifest, out, indent=2)
    out.write("\n")
PY

chrome="${CHROME:-}"
if [ -z "$chrome" ]; then
  for candidate in google-chrome chromium chromium-browser; do
    if command -v "$candidate" >/dev/null; then
      chrome="$candidate"
      break
    fi
  done
fi

if [ -n "$chrome" ] && command -v node >/dev/null; then
  echo "Capturing the Web UI screenshot..."
  start_server "$root/examples/invoice"
  node "$root/website/scripts/capture-webui.mjs" "$chrome" \
    "$base_url/_webui/?template=invoice" \
    "$root/examples/invoice/templates/invoice/payload.json" \
    "$root/website/src/assets/webui.png"
  stop_server
else
  echo "Chrome or node not found; keeping the existing website/src/assets/webui.png"
fi

echo "Done. Review and commit website/public/examples and website/src/assets."
