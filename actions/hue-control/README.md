# Hue Control

An OpenAction plugin for OpenDeck that controls Philips Hue lights and rooms through the Hue Bridge **local API**. It replaces Elgato's official Philips Hue plugin, whose property inspector does not render under OpenDeck: that plugin's config panel only ever shows **Bridge** and **Light**, leaving color, cycle, brightness, temperature, and scene controls unusable.

The root cause is that the Elgato property inspector issues its bridge requests from inside the webview, which the OpenDeck WebKit panel blocks. Hue Control moves every network call into the Rust plugin and has the inspector request data over the OpenAction socket instead, so all seven actions are configurable.

## Actions

| Action | What it does |
| :--- | :--- |
| **On / Off** | Toggles power and reflects the real state on the button. |
| **Color** | Sets a fixed color from a native color picker. |
| **Color Cycle** | Steps through a 2–10 color palette, advancing one color per press. |
| **Brightness** | Sets an absolute brightness (1–100%). |
| **Brightness Steps** | Nudges brightness by a signed step count (−50…+50). |
| **Temperature** | Shifts color temperature between warm and cool. |
| **Scene** | Applies a scene to a group. |

Brightness, Brightness Steps, and Temperature also work on encoders: rotate the dial to adjust the value, press it to toggle power.

## Build

```sh
./scripts/package.sh
```

Then copy the bundle into OpenDeck's plugins directory and restart OpenDeck:

```sh
cp -r plugin/io.github.mario.huecontrol.sdPlugin ~/.config/opendeck/plugins/
```

## Setup (in the property inspector)

1. Press **Discover** to find bridges on the local network, or type the bridge IP by hand.
2. Press the physical link button on the Hue Bridge, then press **Pair**. The username is stored in the plugin's global settings and shared by every instance, so pairing is a one-time step.
3. Pick a **group** (rooms/areas first, since scenes are group-scoped) or an individual **light**.
4. Configure the action-specific control that appears — color, palette, brightness, temperature, or scene.

Bridge credentials use the same global-settings shape as Elgato's plugin (`{"bridges":{"<bridgeid>":{"ip","username"}}}`), so an existing Elgato pairing can be reused by copying `~/.config/opendeck/settings/com.elgato.philips-hue.sdPlugin.json` to `io.github.mario.huecontrol.sdPlugin.json`.

## Notes

- Bridge control traffic is plain HTTP to a local address; the only outbound internet call is the optional `discovery.meethue.com` lookup.
- Failures are surfaced on the button: an unreachable bridge, an unauthorized username, or a missing target shows an alert and an error face rather than reporting a false success.
- Scenes are group-scoped, so selecting one requires a group target.
