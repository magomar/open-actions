<script lang="ts">
  import { onMount } from "svelte";
  import {
    actionSettings,
    actionInfo,
    sendToPlugin,
    eventTarget,
  } from "@openaction/svelte-pi";

  type ProjectItem = {
    id: string;
    name: string;
    path: string;
  };

  const actionUuid = $derived($actionInfo?.action ?? "");
  const isGlobal = $derived(actionUuid.endsWith(".global"));

  let projects = $state<ProjectItem[]>([]);
  let isRunning = $state(false);

  const defaults = {
    apiUrl: "http://127.0.0.1:3000",
    projectId: "",
    fixedProjectId: "",
    refreshIntervalSecs: 10,
    displayMode: "status",
    metricsTimeoutSecs: 0,
  };

  const settings = $derived({ ...defaults, ...$actionSettings });
  const currentProjectId = $derived(settings.projectId || settings.fixedProjectId || "");

  function update<K extends keyof typeof defaults>(key: K, value: typeof defaults[K]) {
    actionSettings.update((saved: any) => ({ ...defaults, ...saved, [key]: value }));
  }

  function onPluginEvent(name: string, handler: (payload: any) => void) {
    eventTarget.addEventListener("sendToPropertyInspector", (event: any) => {
      const payload = event.detail?.payload;
      if (payload?.event === name) {
        handler(payload);
      }
    });
  }

  onPluginEvent("projectsList", (payload) => {
    projects = payload.projects ?? [];
    isRunning = Boolean(payload.isRunning);
  });

  function requestSync() {
    sendToPlugin({ action: "getProjects" });
  }

  onMount(() => {
    requestSync();
    const interval = setInterval(requestSync, 3000);
    return () => clearInterval(interval);
  });
</script>

<div class="container">
  <!-- Status Header Card -->
  <div class="card">
    <div class="row">
      <span style="font-weight: 700; color: #f4f4f5;">
        {isGlobal ? "Fleet Global Overview" : "Fleet Project Monitor"}
      </span>
      {#if isRunning}
        <span class="badge badge-online">● Online ({projects.length})</span>
      {:else}
        <span class="badge badge-offline">○ Offline</span>
      {/if}
    </div>
    {#if !isRunning}
      <p class="hint" style="color: #f87171; margin-top: 6px;">
        Fleet is not running at {settings.apiUrl}. Pressing the key on your device will launch Fleet.
      </p>
    {:else if isGlobal}
      <p class="hint" style="color: #94a3b8; margin-top: 6px;">
        Displays fleet-wide 4-state totals in the 4 corners: 🟢 Clean, 🔵 Ready, 🟡 In Progress, 🔴 Blocked. Pressing the key refreshes metrics.
      </p>
    {/if}
  </div>

  <!-- Global Action Specific Controls -->
  {#if isGlobal}
    <div class="card">
      <div class="field">
        <label for="refreshInterval">Refresh Frequency (Seconds)</label>
        <input
          type="number"
          id="refreshInterval"
          min="1"
          max="3600"
          value={settings.refreshIntervalSecs}
          oninput={(e) => {
            const parsed = parseInt((e.target as HTMLInputElement).value, 10);
            if (!isNaN(parsed) && parsed >= 1) {
              update("refreshIntervalSecs", parsed);
            }
          }}
        />
        <span class="hint">
          Background polling interval for Fleet status and project metrics (minimum: 1s, default: 10s).
        </span>
      </div>
    </div>
  {/if}

  <!-- Project Action Specific Controls -->
  {#if !isGlobal}
    <div class="card">
      <div class="field">
        <label for="project">Target Project</label>
        <select
          id="project"
          value={currentProjectId}
          onchange={(e) => {
            const val = (e.target as HTMLSelectElement).value;
            update("projectId", val);
            update("fixedProjectId", val);
          }}
        >
          <option value="">-- Select a project --</option>
          {#each projects as p}
            <option value={p.id}>{p.name} ({p.path})</option>
          {/each}
        </select>
        {#if projects.length === 0}
          <span class="hint" style="color: #fbbf24;">
            No registered projects found in Fleet.
          </span>
        {:else}
          <span class="hint">
            Pressing this key launches Fleet (if closed) or switches workspace to this project while toggling display modes.
          </span>
        {/if}
      </div>
      <div class="field" style="margin-top: 12px;">
        <label for="displayMode">Display Mode</label>
        <select
          id="displayMode"
          value={settings.displayMode}
          onchange={(e) => {
            const val = (e.target as HTMLSelectElement).value;
            update("displayMode", val);
          }}
        >
          <option value="status">Project Status (Overview)</option>
          <option value="issues">Bead Issues (4-Corner Breakdown)</option>
        </select>
        <span class="hint">
          Clicking the key on the deck toggles between Status Overview and Bead Issues breakdown.
        </span>
      </div>
      <div class="field" style="margin-top: 12px;">
        <label for="metricsTimeout">Metrics Timeout (Seconds)</label>
        <input
          type="number"
          id="metricsTimeout"
          min="0"
          max="3600"
          value={settings.metricsTimeoutSecs}
          oninput={(e) => {
            const parsed = parseInt((e.target as HTMLInputElement).value, 10);
            update("metricsTimeoutSecs", isNaN(parsed) || parsed < 0 ? 0 : parsed);
          }}
        />
        <span class="hint">
          Seconds before returning to project status overview (0 = stay until clicked again).
        </span>
      </div>
    </div>
  {/if}

  <!-- Common Connection Settings Card -->
  <div class="card">
    <div class="field">
      <label for="apiUrl">Fleet Endpoint URL</label>
      <input
        type="text"
        id="apiUrl"
        value={settings.apiUrl}
        oninput={(e) => update("apiUrl", (e.target as HTMLInputElement).value)}
      />
      <span class="hint">Default local daemon is http://127.0.0.1:3000</span>
    </div>
    <div class="row" style="margin-top: 8px;">
      <button type="button" class="btn" onclick={requestSync}>
        Refresh Telemetry
      </button>
    </div>
  </div>
</div>
