#!/usr/bin/env sh
set -eu

cargo build --release
cp target/release/hue-control plugin/io.github.mario.huecontrol.sdPlugin/hue-control
(cd pi && npm run build)

# Rebuild the distributable zip from the bundle just produced. Skipping this
# leaves a stale zip next to a fresh bundle, and installing that zip silently
# reverts the plugin to older code.
BUNDLE=plugin/io.github.mario.huecontrol.sdPlugin
VERSION=$(sed -n 's/.*"Version": *"\([^"]*\)".*/\1/p' "$BUNDLE/manifest.json")
ZIP="plugin/hue-control-$VERSION.zip"

rm -f plugin/hue-control-*.zip
python3 - "$BUNDLE" "$ZIP" <<'PY'
import pathlib, sys, zipfile

bundle, out = pathlib.Path(sys.argv[1]), sys.argv[2]
# Archive paths must be relative to the bundle's PARENT, so the zip root is the
# .sdPlugin folder itself. Keeping the "plugin/" prefix would make OpenDeck
# extract into plugins/plugin/<uuid>/ and silently install nothing.
root = bundle.parent
files = sorted(path for path in bundle.rglob("*") if path.is_file())
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as archive:
    for path in files:
        archive.write(path, path.relative_to(root).as_posix())
print(f"Wrote {out} ({len(files)} files)")
PY

printf 'Plugin bundle: %s\n' "$BUNDLE"
