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
  const isDaemonAction = $derived(actionUuid.endsWith(".toggle-daemon"));

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
        {isDaemonAction
          ? "Toggle Background Service"
          : isWindowAction
            ? "Record Window"
            : "Record Screen"}
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
      {#if isDaemonAction}
        Starts the background <code>gsr-ui</code> daemon if stopped, or stops active recordings and kills the background GPU Screen Recorder process if running.
      {:else if isWindowAction}
        Automatically starts or stops recording the currently focused window without a portal picker dialog.
      {:else}
        Starts or stops whole-screen recording using your <code>gsr-ui</code> monitor and audio settings.
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
