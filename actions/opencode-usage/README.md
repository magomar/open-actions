# OpenCode Usage

An OpenAction plugin for OpenDeck that displays OpenCode Go's 5-hour, weekly, or monthly usage on a button.

## Build

```sh
./scripts/package.sh
```

Copy `plugin/io.github.mario.opencodeusage.sdPlugin` into OpenDeck's `plugins` directory, then restart OpenDeck. Select the action in OpenDeck to configure its OpenCode Go API key and display mode: a single fixed metric, cycling on press, or cycling every chosen number of minutes.

Zen balance is not included: OpenCode does not expose a key-authenticated balance endpoint yet ([anomalyco/opencode#10448](https://github.com/anomalyco/opencode/issues/10448)).
