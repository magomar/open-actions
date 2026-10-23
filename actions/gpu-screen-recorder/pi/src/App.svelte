<script lang="ts">
  import { onMount } from "svelte";
  import {
    actionInfo,
    actionSettings,
    eventTarget,
    sendToPlugin,
  } from "@openaction/svelte-pi";

  const defaults = {
    gsr_ui_cli_path: "",
  };

  const actionUuid = $derived($actionInfo?.action ?? "");
  const isWindowAction = $derived(actionUuid.endsWith(".record-window"));

  let daemonRunning = $state(false);
  let recording = $state(false);
  let captureMode = $state<string | null>(null);
  let elapsedSecs = $state<number | null>(null);

  const settings = $derived({ ...defaults, ...$actionSettings });

  function update(key: keyof typeof defaults, value: string) {
    actionSettings.update((saved: any) => ({ ...defaults, ...saved, [key]: value }));
  }

  function requestStatus() {
    sendToPlugin({ event: "getStatus" });
  }

  onMount(() => {
    const handler = (event: any) => {
      const payload = event.detail?.payload;
      if (payload?.event === "status") {
        daemonRunning = Boolean(payload.daemonRunning);
        recording = Boolean(payload.recording);
        captureMode = payload.captureMode ?? null;
        elapsedSecs = typeof payload.elapsedSecs === "number" ? payload.elapsedSecs : null;
      }
    };

    eventTarget.addEventListener("sendToPropertyInspector", handler);
    requestStatus();
    const interval = setInterval(requestStatus, 2000);
    return () => {
      clearInterval(interval);
      eventTarget.removeEventListener("sendToPropertyInspector", handler);
    };
  });
</script>

<div class="container">
  <div class="card">
    <div class="row">
      <span style="font-weight: 700; color: #f4f4f5;">
        {isWindowAction ? "Record Window" : "Record Screen"}
      </span>
      <div style="display: flex; gap: 6px; align-items: center;">
        {#if recording}
          <span class="badge badge-recording">
            ● REC ({captureMode ?? "active"})
          </span>
        {:else if daemonRunning}
          <span class="badge badge-online">● gsr-ui Ready</span>
        {:else}
          <span class="badge badge-offline">○ gsr-ui Offline</span>
        {/if}
        <button class="btn" type="button" onclick={requestStatus}>Check Status</button>
      </div>
    </div>
    <p class="hint">
      {#if isWindowAction}
        Toggles window recording via <code>gsr-ui-cli toggle-record-window</code> (opens the XDG Desktop Portal window picker on Wayland, or selects a window on X11).
      {:else}
        Toggles whole-screen recording via <code>gsr-ui-cli toggle-record</code> using your <code>gsr-ui</code> monitor and audio settings.
      {/if}
    </p>
  </div>

  <div class="card">
    <div class="field">
      <label for="gsr-ui-cli-path">gsr-ui-cli Binary Path</label>
      <input
        id="gsr-ui-cli-path"
        type="text"
        placeholder="gsr-ui-cli"
        value={settings.gsr_ui_cli_path ?? ""}
        oninput={(event) => update("gsr_ui_cli_path", event.currentTarget.value)}
      />
      <p class="hint">
        Leave empty to use <code>gsr-ui-cli</code> on <code>PATH</code>.
      </p>
    </div>
  </div>
</div>
