#!/usr/bin/env sh
set -eu

cargo build --release
cp target/release/handy-transcribe plugin/io.github.mario.handytranscribe.sdPlugin/handy-transcribe
(cd pi && npm run build)
printf 'Plugin bundle: %s\n' "plugin/io.github.mario.handytranscribe.sdPlugin"
