#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

cargo build --release
cp target/release/opencode-usage plugin/io.github.mario.opencodeusage.sdPlugin/opencode-usage
chmod +x plugin/io.github.mario.opencodeusage.sdPlugin/opencode-usage
(cd pi && npm run build)

BUNDLE=plugin/io.github.mario.opencodeusage.sdPlugin
VERSION=$(sed -n 's/.*"Version": *"\([^"]*\)".*/\1/p' "$BUNDLE/manifest.json")
ZIP="plugin/opencode-usage-$VERSION.zip"

rm -f plugin/opencode-usage-*.zip
python3 - "$BUNDLE" "$ZIP" <<'PY'
import pathlib, sys, zipfile

bundle, out = pathlib.Path(sys.argv[1]), sys.argv[2]
root = bundle.parent
files = sorted(path for path in bundle.rglob("*") if path.is_file())
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as archive:
    for path in files:
        archive.write(path, path.relative_to(root).as_posix())
print(f"Wrote {out} ({len(files)} files)")
PY

printf 'Plugin bundle: %s\n' "$BUNDLE"
