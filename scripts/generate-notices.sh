#!/usr/bin/env bash
# Generates the third-party license notices for the typrest binary.
#
# Requires cargo-about (cargo install cargo-about --locked --features cli). The output also embeds
# the NOTICE file shipped with typst-assets, which carries the licenses of the bundled fonts and
# data; cargo-about does not pick that file up on its own.
#
# Usage: scripts/generate-notices.sh [output-file]   (default: target/THIRD_PARTY_NOTICES.html)
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
OUT="${1:-target/THIRD_PARTY_NOTICES.html}"
mkdir -p "$(dirname "$OUT")"

cargo about generate --locked --manifest-path crates/typrest/Cargo.toml about.hbs -o "$OUT"

ASSETS_DIR="$(cargo metadata --locked --format-version 1 |
    python3 -c 'import json, sys
for p in json.load(sys.stdin)["packages"]:
    if p["name"] == "typst-assets":
        print(p["manifest_path"].rsplit("/", 1)[0])')"
NOTICE="$ASSETS_DIR/NOTICE"
[[ -f "$NOTICE" ]] || { echo "typst-assets NOTICE not found at $NOTICE" >&2; exit 1; }

python3 - "$OUT" "$NOTICE" <<'PY'
import html, sys
out, notice = sys.argv[1:]
page = open(out, encoding="utf-8").read()
section = ("<h2>typst-assets: bundled fonts and data</h2>\n<pre>"
           + html.escape(open(notice, encoding="utf-8").read()) + "</pre>\n")
page = page.replace("</body>", section + "</body>", 1)
open(out, "w", encoding="utf-8").write(page)
PY
echo "Wrote $OUT"
