#!/usr/bin/env bash
set -eu

ACTION="${1:-}"
if [ -z "$ACTION" ]; then
    echo "Usage: $0 <action-name>"
    echo "Available actions:"
    ls -1 actions
    exit 1
fi

TARGET_DIR="actions/$ACTION"
if [ ! -d "$TARGET_DIR" ]; then
    echo "Error: Action '$ACTION' not found in actions/."
    echo "Available actions:"
    ls -1 actions
    exit 1
fi

if [ -x "$TARGET_DIR/scripts/package.sh" ]; then
    echo "==> Packaging $ACTION via $TARGET_DIR/scripts/package.sh..."
    (cd "$TARGET_DIR" && ./scripts/package.sh)
elif [ -f "$TARGET_DIR/scripts/package.sh" ]; then
    echo "==> Packaging $ACTION via $TARGET_DIR/scripts/package.sh..."
    (cd "$TARGET_DIR" && sh ./scripts/package.sh)
else
    echo "==> Packaging $ACTION via standard workflow..."
    cargo build --release --manifest-path "$TARGET_DIR/Cargo.toml"
    BUNDLE=$(find "$TARGET_DIR/plugin" -maxdepth 1 -type d -name "*.sdPlugin" | head -n 1)
    if [ -z "$BUNDLE" ]; then
        echo "Error: No *.sdPlugin directory found in $TARGET_DIR/plugin"
        exit 1
    fi
    BIN_NAME=$(basename "$TARGET_DIR")
    cp "$TARGET_DIR/target/release/$BIN_NAME" "$BUNDLE/$BIN_NAME"
    chmod +x "$BUNDLE/$BIN_NAME"
    if [ -d "$TARGET_DIR/pi" ]; then
        if command -v bun >/dev/null 2>&1 && [ -f "$TARGET_DIR/pi/bun.lock" ]; then
            (cd "$TARGET_DIR/pi" && bun run build)
        elif [ -f "$TARGET_DIR/pi/package.json" ]; then
            (cd "$TARGET_DIR/pi" && npm run build)
        fi
    fi
    VERSION=$(sed -n 's/.*"Version": *"\([^"]*\)".*/\1/p' "$BUNDLE/manifest.json")
    ZIP="$TARGET_DIR/plugin/$BIN_NAME-$VERSION.zip"
    rm -f "$TARGET_DIR"/plugin/"$BIN_NAME"-*.zip
    python3 - "$BUNDLE" "$ZIP" << 'PY'
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
fi
