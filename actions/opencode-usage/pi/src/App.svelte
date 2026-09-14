<script lang="ts">
  import { actionSettings } from "@openaction/svelte-pi";

  type Metric = "go_5h" | "go_weekly" | "go_monthly";
  type DisplayMode = "fixed" | "cycle_manual" | "cycle_periodic";

  const defaults = {
    api_key: "",
    display_mode: "fixed" as DisplayMode,
    fixed_metric: "go_5h" as Metric,
    cycle_interval_seconds: 300,
  };

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
    <label class="sdpi-item-label" for="api-key">API key</label>
    <input class="sdpi-item-value" id="api-key" type="password" value={$actionSettings.api_key ?? ""} oninput={(event) => update("api_key", event.currentTarget.value)} />
  </div>
  <div class="sdpi-item">
    <label class="sdpi-item-label" for="display-mode">Display mode</label>
    <select class="sdpi-item-value" id="display-mode" value={$actionSettings.display_mode ?? defaults.display_mode} onchange={(event) => update("display_mode", event.currentTarget.value as DisplayMode)}>
      <option value="fixed">Single fixed metric</option>
      <option value="cycle_manual">Cycle on press</option>
      <option value="cycle_periodic">Cycle every X minutes</option>
    </select>
  </div>
  {#if ($actionSettings.display_mode ?? defaults.display_mode) === "fixed"}
    <div class="sdpi-item">
      <label class="sdpi-item-label" for="fixed-metric">Metric</label>
      <select class="sdpi-item-value" id="fixed-metric" value={$actionSettings.fixed_metric ?? defaults.fixed_metric} onchange={(event) => update("fixed_metric", event.currentTarget.value as Metric)}>
        <option value="go_5h">5-hour usage</option>
        <option value="go_weekly">Weekly usage</option>
        <option value="go_monthly">Monthly usage</option>
      </select>
    </div>
  {:else if ($actionSettings.display_mode ?? defaults.display_mode) === "cycle_periodic"}
    <div class="sdpi-item">
      <label class="sdpi-item-label" for="cycle-interval">Interval (minutes)</label>
      <input class="sdpi-item-value" id="cycle-interval" type="number" min="1" value={Math.max(1, ($actionSettings.cycle_interval_seconds ?? defaults.cycle_interval_seconds) / 60)} oninput={(event) => updateInterval(Number(event.currentTarget.value))} />
    </div>
  {/if}
  <p class="sdpi-note">Usage is displayed on the selected button.</p>
</main>
