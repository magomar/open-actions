<script lang="ts">
  import { actionSettings, sendToPlugin } from "@openaction/svelte-pi";

  interface Settings {
    focus_duration_mins: number;
    short_break_duration_mins: number;
    long_break_duration_mins: number;
    rounds: number;
    auto_start_breaks: boolean;
    auto_start_focus: boolean;
    sound_enabled: boolean;
    volume: number;
    theme: string;
    custom_focus_color?: string;
    custom_short_break_color?: string;
    custom_long_break_color?: string;
  }

  const defaults: Settings = {
    focus_duration_mins: 25,
    short_break_duration_mins: 5,
    long_break_duration_mins: 15,
    rounds: 4,
    auto_start_breaks: false,
    auto_start_focus: false,
    sound_enabled: true,
    volume: 80,
    theme: "classic",
    custom_focus_color: "#ef5350",
    custom_short_break_color: "#4ade80",
    custom_long_break_color: "#22d3ee",
  };

  function update<K extends keyof Settings>(key: K, value: Settings[K]) {
    actionSettings.update((saved) => ({ ...defaults, ...saved, [key]: value }));
  }

  function resetToDefaults() {
    actionSettings.set({ ...defaults });
    sendToPlugin({ event: "reset_defaults" });
  }
</script>

<main class="sdpi-wrapper">
  <div class="sdpi-heading">Timer Durations</div>

  <div class="sdpi-item">
    <label class="sdpi-item-label" for="focus-duration">Focus (min)</label>
    <input
      class="sdpi-item-value"
      id="focus-duration"
      type="number"
      min="1"
      max="90"
      value={$actionSettings.focus_duration_mins ?? defaults.focus_duration_mins}
      oninput={(e) => update("focus_duration_mins", Math.max(1, Number(e.currentTarget.value) || 1))}
    />
  </div>

  <div class="sdpi-item">
    <label class="sdpi-item-label" for="short-break">Short Break (min)</label>
    <input
      class="sdpi-item-value"
      id="short-break"
      type="number"
      min="1"
      max="30"
      value={$actionSettings.short_break_duration_mins ?? defaults.short_break_duration_mins}
      oninput={(e) => update("short_break_duration_mins", Math.max(1, Number(e.currentTarget.value) || 1))}
    />
  </div>

  <div class="sdpi-item">
    <label class="sdpi-item-label" for="long-break">Long Break (min)</label>
    <input
      class="sdpi-item-value"
      id="long-break"
      type="number"
      min="1"
      max="60"
      value={$actionSettings.long_break_duration_mins ?? defaults.long_break_duration_mins}
      oninput={(e) => update("long_break_duration_mins", Math.max(1, Number(e.currentTarget.value) || 1))}
    />
  </div>

  <div class="sdpi-item">
    <label class="sdpi-item-label" for="rounds">Rounds</label>
    <input
      class="sdpi-item-value"
      id="rounds"
      type="number"
      min="1"
      max="12"
      value={$actionSettings.rounds ?? defaults.rounds}
      oninput={(e) => update("rounds", Math.max(1, Number(e.currentTarget.value) || 1))}
    />
  </div>

  <div class="sdpi-heading">Automation</div>

  <div class="sdpi-item">
    <label class="sdpi-item-label" for="auto-start-breaks">Auto-start Breaks</label>
    <input
      class="sdpi-item-value"
      id="auto-start-breaks"
      type="checkbox"
      checked={$actionSettings.auto_start_breaks ?? defaults.auto_start_breaks}
      onchange={(e) => update("auto_start_breaks", e.currentTarget.checked)}
    />
  </div>

  <div class="sdpi-item">
    <label class="sdpi-item-label" for="auto-start-focus">Auto-start Focus</label>
    <input
      class="sdpi-item-value"
      id="auto-start-focus"
      type="checkbox"
      checked={$actionSettings.auto_start_focus ?? defaults.auto_start_focus}
      onchange={(e) => update("auto_start_focus", e.currentTarget.checked)}
    />
  </div>

  <div class="sdpi-heading">Sound & Alerts</div>

  <div class="sdpi-item">
    <label class="sdpi-item-label" for="sound-enabled">Sound Alerts</label>
    <input
      class="sdpi-item-value"
      id="sound-enabled"
      type="checkbox"
      checked={$actionSettings.sound_enabled ?? defaults.sound_enabled}
      onchange={(e) => update("sound_enabled", e.currentTarget.checked)}
    />
  </div>

  {#if ($actionSettings.sound_enabled ?? defaults.sound_enabled)}
    <div class="sdpi-item">
      <label class="sdpi-item-label" for="volume">Volume ({$actionSettings.volume ?? defaults.volume}%)</label>
      <input
        class="sdpi-item-value"
        id="volume"
        type="range"
        min="0"
        max="100"
        value={$actionSettings.volume ?? defaults.volume}
        oninput={(e) => update("volume", Number(e.currentTarget.value))}
      />
    </div>
  {/if}

  <div class="sdpi-heading">Theme</div>

  <div class="sdpi-item">
    <label class="sdpi-item-label" for="theme">Preset</label>
    <select
      class="sdpi-item-value"
      id="theme"
      value={$actionSettings.theme ?? defaults.theme}
      onchange={(e) => update("theme", e.currentTarget.value)}
    >
      <option value="classic">Pomotroid Classic</option>
      <option value="gruvbox">Gruvbox</option>
      <option value="nord">Nord</option>
      <option value="catppuccin">Catppuccin Mocha</option>
      <option value="tokyo_night">Tokyo Night</option>
      <option value="monokai">Monokai</option>
      <option value="custom">Custom Colors</option>
    </select>
  </div>

  {#if ($actionSettings.theme ?? defaults.theme) === "custom"}
    <div class="sdpi-item">
      <label class="sdpi-item-label" for="custom-focus">Focus Color</label>
      <input
        class="sdpi-item-value"
        id="custom-focus"
        type="color"
        value={$actionSettings.custom_focus_color ?? defaults.custom_focus_color}
        oninput={(e) => update("custom_focus_color", e.currentTarget.value)}
      />
    </div>
    <div class="sdpi-item">
      <label class="sdpi-item-label" for="custom-short">Short Break Color</label>
      <input
        class="sdpi-item-value"
        id="custom-short"
        type="color"
        value={$actionSettings.custom_short_break_color ?? defaults.custom_short_break_color}
        oninput={(e) => update("custom_short_break_color", e.currentTarget.value)}
      />
    </div>
    <div class="sdpi-item">
      <label class="sdpi-item-label" for="custom-long">Long Break Color</label>
      <input
        class="sdpi-item-value"
        id="custom-long"
        type="color"
        value={$actionSettings.custom_long_break_color ?? defaults.custom_long_break_color}
        oninput={(e) => update("custom_long_break_color", e.currentTarget.value)}
      />
    </div>
  {/if}

  <button class="sdpi-button" onclick={resetToDefaults}>
    Reset Defaults
  </button>
</main>
