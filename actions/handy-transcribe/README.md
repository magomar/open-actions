# Handy Transcribe

An OpenAction plugin for OpenDeck that toggles [Handy](https://handy.computer) transcription from a button.

## Build

```sh
./scripts/package.sh
```

Copy `plugin/io.github.mario.handytranscribe.sdPlugin` into OpenDeck's `plugins` directory, then restart OpenDeck. Press the button to start or stop Handy transcription; the icon turns teal with a red dot while recording.

The button drives Handy through its remote-control CLI (`handy --toggle-transcription`), so Handy must be installed and running. Leave "Handy binary path" empty to auto-detect (the macOS app bundle, or `handy` on PATH), or set the full path to the Handy binary.

The recording state is optimistic: it reflects the button's own toggle, not Handy's true state (Handy exposes no live status API). If transcription is started or stopped elsewhere — its hotkey, or `Escape` — the icon can drift.
