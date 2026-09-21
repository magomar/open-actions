#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

cargo build --release
cp target/release/pomodoro-timer plugin/io.github.mario.pomodorotimer.sdPlugin/pomodoro-timer
chmod +x plugin/io.github.mario.pomodorotimer.sdPlugin/pomodoro-timer
(cd pi && npm run build)

BUNDLE=plugin/io.github.mario.pomodorotimer.sdPlugin
VERSION=$(sed -n 's/.*"Version": *"\([^"]*\)".*/\1/p' "$BUNDLE/manifest.json")
ZIP="plugin/pomodoro-timer-$VERSION.zip"

rm -f plugin/pomodoro-timer-*.zip
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
