#!/usr/bin/env sh
set -eu

cargo build --release
cp target/release/opencode-usage plugin/io.github.mario.opencodeusage.sdPlugin/opencode-usage
(cd pi && npm run build)
printf 'Plugin bundle: %s\n' "plugin/io.github.mario.opencodeusage.sdPlugin"
