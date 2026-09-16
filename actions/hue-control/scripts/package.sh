#!/usr/bin/env sh
set -eu

cargo build --release
cp target/release/hue-control plugin/io.github.mario.huecontrol.sdPlugin/hue-control
(cd pi && npm run build)
printf 'Plugin bundle: %s\n' "plugin/io.github.mario.huecontrol.sdPlugin"
