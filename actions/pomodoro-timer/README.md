# Pomodoro Timer OpenAction Plugin ⏱️

An OpenAction plugin for OpenDeck and compatible devices (Elgato Stream Deck, Tacto) based on the Pomodoro Technique and inspired by the visual design and minimalist workflow of **Pomotroid**.

Spec: [`specs/006_pomodoro_timer.md`](../../specs/006_pomodoro_timer.md)

## ✨ Features

- **Pomotroid-inspired Visuals**: Dynamic 144×144 SVG circular dial gauges with live progress ring, crisp `MM:SS` countdown timer, uppercase phase badges (`FOCUS`, `SHORT BREAK`, `LONG BREAK`), and fractional round indicators (`1/4` → `4/4`).
- **Phase Color Coding**:
  - 🔴 **Focus**: Coral Red (`#ef5350`)
  - 🟢 **Short Break**: Mint Green (`#4ade80`)
  - 🔵 **Long Break**: Teal / Cyan (`#22d3ee`)
  - ⚪ **Paused / Idle**: Muted Slate (`#64748b`) with Play glyph
- **Physical Key Controls**:
  - **Short Press**: Start / Pause countdown
  - **Long Press (>600ms)**: Skip to next phase
  - **Double Press**: Reset current phase
- **Encoder Dial Support (Stream Deck + / Tacto)**:
  - **Rotate**: Adjust timer duration (+/- 1 min)
  - **Dial Press**: Toggle Start / Pause
  - **Touch Strip**: Full width progress bar and controls
- **Companion Actions**:
  - Dedicated **Skip** and **Reset** buttons for multi-key deck layouts.
- **Audio Chimes**: Pleasant chimes on phase completion with volume adjustment.
- **Svelte Property Inspector**: Customizable durations (Focus, Short Break, Long Break, Rounds), "Reset Defaults" button, auto-start toggles, and color theme presets (*Pomotroid Classic*, *Gruvbox*, *Nord*, *Catppuccin*, *Tokyo Night*, *Custom*).

## 🔨 Building & Packaging

```bash
# Build Rust binary & Svelte property inspector, package .sdPlugin zip
./scripts/package.sh
```
