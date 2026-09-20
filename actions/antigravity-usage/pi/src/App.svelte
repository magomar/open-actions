<script lang="ts">
  import { actionSettings, globalSettings } from "@openaction/svelte-pi";

  type Metric = "gemini_quota" | "claude_quota" | "prompt_credits" | "flow_credits";
  type DisplayMode = "fixed" | "cycle_manual" | "cycle_periodic";
  type DisplayFractionAs = "remaining" | "used";

  const defaults = {
    display_mode: "fixed" as DisplayMode,
    fixed_metric: "gemini_quota" as Metric,
    cycle_interval_seconds: 300,
    display_fraction_as: "remaining" as DisplayFractionAs,
    override_port: null as number | null,
    override_csrf_token: "",
  };

  const globalPort = $derived($globalSettings?.override_port ?? null);
  const globalCsrf = $derived($globalSettings?.override_csrf_token ?? "");

  function update(key: keyof typeof defaults, value: unknown) {
    actionSettings.update((saved) => ({ ...defaults, ...saved, [key]: value }));
  }

  function updateInterval(minutes: number) {
    update(
      "cycle_interval_seconds",
      Math.max(1, Number.isFinite(minutes) ? Math.floor(minutes) : 1) * 60,
    );
  }
</script>

<main class="sdpi-wrapper">
  <div class="sdpi-item">
    <label class="sdpi-item-label" for="display-mode">Display mode</label>
    <select
      class="sdpi-item-value"
      id="display-mode"
      value={$actionSettings.display_mode ?? defaults.display_mode}
      onchange={(event) => update("display_mode", event.currentTarget.value as DisplayMode)}
    >
      <option value="fixed">Single fixed metric</option>
      <option value="cycle_manual">Cycle on press</option>
      <option value="cycle_periodic">Cycle every X minutes</option>
    </select>
  </div>

  {#if ($actionSettings.display_mode ?? defaults.display_mode) === "fixed"}
    <div class="sdpi-item">
      <label class="sdpi-item-label" for="fixed-metric">Metric</label>
      <select
        class="sdpi-item-value"
        id="fixed-metric"
        value={$actionSettings.fixed_metric ?? defaults.fixed_metric}
        onchange={(event) => update("fixed_metric", event.currentTarget.value as Metric)}
      >
        <option value="gemini_quota">Gemini Quota</option>
        <option value="claude_quota">Claude Quota</option>
        <option value="prompt_credits">Prompt Credits</option>
        <option value="flow_credits">Flow Credits</option>
      </select>
    </div>
  {:else if ($actionSettings.display_mode ?? defaults.display_mode) === "cycle_periodic"}
    <div class="sdpi-item">
      <label class="sdpi-item-label" for="cycle-interval">Interval (minutes)</label>
      <input
        class="sdpi-item-value"
        id="cycle-interval"
        type="number"
        min="1"
        value={Math.max(1, ($actionSettings.cycle_interval_seconds ?? defaults.cycle_interval_seconds) / 60)}
        oninput={(event) => updateInterval(Number(event.currentTarget.value))}
      />
    </div>
  {/if}

  <div class="sdpi-item">
    <label class="sdpi-item-label" for="display-fraction">Values</label>
    <select
      class="sdpi-item-value"
      id="display-fraction"
      value={$actionSettings.display_fraction_as ?? defaults.display_fraction_as}
      onchange={(event) => update("display_fraction_as", event.currentTarget.value as DisplayFractionAs)}
    >
      <option value="remaining">Remaining % (capacity style)</option>
      <option value="used">Used % (spend style)</option>
    </select>
  </div>

  <p class="sdpi-note">
    Antigravity Language Server is auto-detected from /proc. Leave override fields blank for zero-config discovery.
  </p>

  <div class="sdpi-item">
    <label class="sdpi-item-label" for="override-port">Override Port</label>
    <input
      class="sdpi-item-value"
      id="override-port"
      type="number"
      placeholder="Auto"
      value={$actionSettings.override_port ?? globalPort ?? ""}
      oninput={(event) => {
        const val = event.currentTarget.value.trim();
        update("override_port", val ? Number(val) : null);
      }}
    />
  </div>

  <div class="sdpi-item">
    <label class="sdpi-item-label" for="override-csrf">CSRF Token</label>
    <input
      class="sdpi-item-value"
      id="override-csrf"
      type="password"
      placeholder="Auto"
      value={$actionSettings.override_csrf_token ?? globalCsrf ?? ""}
      oninput={(event) => update("override_csrf_token", event.currentTarget.value.trim())}
    />
  </div>
</main>
