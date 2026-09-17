<script lang="ts">
  import {
    actionSettings,
    globalSettings,
    eventTarget,
    actionInfo,
    sendToPlugin,
  } from "@openaction/svelte-pi";

  // Every action shares one settings shape, so the inspector has one default set.
  const defaults = {
    bridge: "",
    target: "",
    mode: "fixed",
    color: "#ffcc66",
    colors: ["#ff0000", "#00ff00", "#0000ff"],
    brightness: 100,
    brightnesses: [25, 50, 75, 100],
    scale_ticks: 1,
    temperature: 50,
    temperatures: [20, 50, 80],
    brightness_rel: 10,
    scene: "",
    scenes: [],
  };

  type Bridge = { ip: string; username: string };
  type Group = { id: string; name: string };
  type Light = { id: string; name: string };

  // The action kind is the last segment of the manifest UUID.
  const rawKind = $derived(($actionInfo?.action ?? "").split(".").at(-1) ?? "");
  const kind = $derived(
    rawKind === "power" ? "switch" :
    rawKind === "cycle" ? "color" :
    rawKind === "brightness-rel" ? "brightness" :
    rawKind
  );

  let discovered = $state<{ id: string; internalipaddress: string }[]>([]);
  let groups = $state<Group[]>([]);
  let lights = $state<Light[]>([]);
  let scenes = $state<{ id: string; name: string; group: string }[]>([]);
  let status = $state("");
  let statusIsError = $state(false);
  let busy = $state(false);
  let pairing = $state(false);
  let manualIp = $state("");
  let pairTimer: ReturnType<typeof setInterval> | null = null;

  const settings = $derived({ ...defaults, ...$actionSettings });
  const bridges = $derived<Record<string, Bridge>>($globalSettings?.bridges ?? {});
  const bridgeIds = $derived(Object.keys(bridges));
  const activeBridge = $derived(bridges[settings.bridge] ?? Object.values(bridges)[0]);
  const paired = $derived(bridgeIds.length > 0);

  const targetValue = $derived(
    (settings.target ?? "").startsWith("g-")
      ? `g-${settings.target.slice(2)}`
      : (settings.target ?? ""),
  );

  // Scenes belong to groups, so the scene picker follows the selected group.
  const groupScenes = $derived(
    targetValue.startsWith("g-")
      ? scenes.filter((scene) => scene.group === targetValue.slice(2))
      : [],
  );

  function update(key: keyof typeof defaults, value: unknown) {
    actionSettings.update((saved: any) => ({ ...defaults, ...saved, [key]: value }));
  }

  function say(message: string, isError = false) {
    status = message;
    statusIsError = isError;
  }

  async function requestTargets(ip?: string, username?: string) {
    const bridge = activeBridge;
    const resolvedIp = ip ?? bridge?.ip;
    const resolvedUser = username ?? bridge?.username;
    if (!resolvedIp || !resolvedUser) {
      return;
    }
    busy = true;
    sendToPlugin({ event: "getTargets", ip: resolvedIp, username: resolvedUser });
  }

  $effect(() => {
    if (activeBridge?.ip && activeBridge?.username) {
      requestTargets();
    }
  });

  // OpenDeck relays everything the plugin sends as one "sendToPropertyInspector"
  // message whose payload carries the real event name, so listening for "paired"
  // directly never fires. Subscribe to the relay and dispatch on the inner name.
  function onPluginEvent(name: string, handler: (payload: any) => void) {
    eventTarget.addEventListener("sendToPropertyInspector", (event: any) => {
      const payload = event.detail?.payload;
      if (payload?.event === name) {
        handler(payload);
      }
    });
  }

  onPluginEvent("bridges", (payload) => {
    discovered = payload.bridges ?? [];
    busy = false;
    say(
      discovered.length
        ? `Found ${discovered.length} bridge(s). Select one, then press its link button and Pair.`
        : "No bridges discovered. Enter the IP address manually.",
      !discovered.length,
    );
  });

  onPluginEvent("paired", (payload) => {
    const { ip, id, username } = payload;
    stopPairing("");
    if (!ip || !username) {
      say("Pairing returned no username.", true);
      return;
    }
    globalSettings.update((saved: any) => ({
      ...saved,
      bridges: { ...(saved?.bridges ?? {}), [id || ip]: { ip, username } },
    }));
    update("bridge", id || ip);
    say(`Paired ${ip}.`);
    requestTargets(ip, username);
  });

  onPluginEvent("pairError", (payload) => {
    stopPairing(payload.message ?? "Pairing failed.", true);
  });

  onPluginEvent("targets", (payload) => {
    const targets = payload.targets ?? {};
    groups = targets.groups ?? [];
    lights = targets.lights ?? [];
    scenes = targets.scenes ?? [];
    busy = false;
    say(`${groups.length} room(s), ${lights.length} light(s), ${scenes.length} scene(s).`);
  });

  onPluginEvent("targetError", (payload) => {
    busy = false;
    say(payload.message ?? "Could not reach the bridge.", true);
  });

  function discover() {
    busy = true;
    say("Searching for bridges…");
    sendToPlugin({ event: "discover" });
  }

  function stopPairing(message: string, isError = false) {
    if (pairTimer !== null) {
      clearInterval(pairTimer);
      pairTimer = null;
    }
    pairing = false;
    busy = false;
    if (message) {
      say(message, isError);
    }
  }

  function pair() {
    const ip = manualIp.trim() || discovered[0]?.internalipaddress || activeBridge?.ip;
    if (!ip) {
      say("Enter the bridge IP address first.", true);
      return;
    }
    busy = true;
    pairing = true;
    const deadline = Date.now() + 120_000;
    // The bridge only accepts a pairing request for ~30s after the link button is pressed,
    // so keep asking until it succeeds rather than making the user race a single attempt.
    const attempt = () => {
      if (!pairing) {
        return;
      }
      const left = Math.ceil((deadline - Date.now()) / 1000);
      if (left <= 0) {
        stopPairing("No link-button press detected. Press the round button, then Pair again.", true);
        return;
      }
      say(`Waiting for the link button — press the round button on top of the bridge (${left}s).`);
      sendToPlugin({ event: "pair", ip });
    };
    attempt();
    pairTimer = setInterval(attempt, 3000);
  }

  function selectBridge(id: string) {
    update("bridge", id);
    requestTargets(bridges[id]?.ip, bridges[id]?.username);
  }

  function removeBridge(id: string) {
    globalSettings.update((saved: any) => {
      const next = { ...(saved?.bridges ?? {}) };
      delete next[id];
      return { ...saved, bridges: next };
    });
    if (settings.bridge === id) {
      update("bridge", "");
    }
  }

  // Cycle palette editing.
  function setColorAt(index: number, value: string) {
    const next = [...settings.colors];
    next[index] = value;
    update("colors", next);
  }

  function addColor() {
    if (settings.colors.length >= 10) {
      return;
    }
    update("colors", [...settings.colors, "#ffffff"]);
  }

  function removeColorAt(index: number) {
    if (settings.colors.length <= 2) {
      return;
    }
    update(
      "colors",
      settings.colors.filter((_: string, at: number) => at !== index),
    );
  }

  function warmthToKelvin(warmth: number): number {
    return Math.round(2000 + ((Math.max(1, Math.min(100, warmth)) - 1) * 4500) / 99);
  }

  function setTemperatureAt(index: number, value: number) {
    const next = [...settings.temperatures];
    next[index] = value;
    update("temperatures", next);
  }

  function addTemperature() {
    if (settings.temperatures.length >= 10) {
      return;
    }
    update("temperatures", [...settings.temperatures, 50]);
  }

  function removeTemperatureAt(index: number) {
    if (settings.temperatures.length <= 2) {
      return;
    }
    update(
      "temperatures",
      settings.temperatures.filter((_: number, at: number) => at !== index),
    );
  }

  function setBrightnessAt(index: number, value: number) {
    const next = [...settings.brightnesses];
    next[index] = value;
    update("brightnesses", next);
  }

  function addBrightness() {
    if (settings.brightnesses.length >= 10) {
      return;
    }
    update("brightnesses", [...settings.brightnesses, 50]);
  }

  function removeBrightnessAt(index: number) {
    if (settings.brightnesses.length <= 2) {
      return;
    }
    update(
      "brightnesses",
      settings.brightnesses.filter((_: number, at: number) => at !== index),
    );
  }

  function setSceneAt(index: number, value: string) {
    const next = [...settings.scenes];
    next[index] = value;
    update("scenes", next);
  }

  function addScene() {
    if (settings.scenes.length >= 10) {
      return;
    }
    const fallback = groupScenes[0]?.id ?? "";
    update("scenes", [...settings.scenes, fallback]);
  }

  function removeSceneAt(index: number) {
    if (settings.scenes.length <= 2) {
      return;
    }
    update(
      "scenes",
      settings.scenes.filter((_: string, at: number) => at !== index),
    );
  }

  const needsTarget = $derived(
    ["switch", "color", "brightness", "temperature", "scene"].includes(kind),
  );
</script>

<main class="sdpi-wrapper">
  <div class="sdpi-heading">Bridge</div>

  {#if paired}
    <div class="sdpi-item">
      <label class="sdpi-item-label" for="bridge-select">Bridge</label>
      <select
        class="sdpi-item-value"
        id="bridge-select"
        value={settings.bridge || bridgeIds[0]}
        onchange={(event) => selectBridge(event.currentTarget.value)}
      >
        {#each bridgeIds as id (id)}
          <option value={id}>{bridges[id].ip}</option>
        {/each}
      </select>
    </div>
    <div class="sdpi-row">
      <button class="sdpi-button" onclick={() => requestTargets()} disabled={busy}>Refresh</button>
      <button class="sdpi-button" onclick={() => removeBridge(settings.bridge || bridgeIds[0])}>
        Forget
      </button>
    </div>
  {:else}
    <div class="sdpi-item">
      <label class="sdpi-item-label" for="bridge-ip">Bridge IP</label>
      <input
        class="sdpi-item-value"
        id="bridge-ip"
        type="text"
        placeholder="192.168.1.74"
        bind:value={manualIp}
      />
    </div>
    <p class="sdpi-note">
      Press the round link button on top of the bridge, then click Pair. The bridge accepts a
      pairing request for about 30 seconds after the button is pressed.
    </p>
    <div class="sdpi-row">
      <button class="sdpi-button" onclick={discover} disabled={busy}>Discover</button>
      {#if pairing}
        <button class="sdpi-button" onclick={() => stopPairing("Pairing cancelled.")}>
          Cancel
        </button>
      {:else}
        <button class="sdpi-button" onclick={pair} disabled={busy}>Pair</button>
      {/if}
    </div>
    {#if discovered.length}
      <div class="sdpi-item">
        <label class="sdpi-item-label" for="discovered-select">Found</label>
        <select class="sdpi-item-value" id="discovered-select" onchange={(event) => (manualIp = event.currentTarget.value)}>
          {#each discovered as bridge (bridge.id)}
            <option value={bridge.internalipaddress}>{bridge.internalipaddress}</option>
          {/each}
        </select>
      </div>
    {/if}
  {/if}

  {#if status}
    <p class="sdpi-status" class:error={statusIsError}>{status}</p>
  {/if}

  {#if paired}
    <div class="sdpi-heading">Target</div>
    <div class="sdpi-item">
      <label class="sdpi-item-label" for="target-select">Light / Group</label>
      <select
        class="sdpi-item-value"
        id="target-select"
        value={targetValue}
        onchange={(event) => update("target", event.currentTarget.value)}
      >
        <option value="">— select —</option>
        {#if groups.length}
          <optgroup label="Groups">
            {#each groups as group (group.id)}
              <option value={`g-${group.id}`}>{group.name}</option>
            {/each}
          </optgroup>
        {/if}
        {#if lights.length}
          <optgroup label="Lights">
            {#each lights as light (light.id)}
              <option value={`l-${light.id}`}>{light.name}</option>
            {/each}
          </optgroup>
        {/if}
      </select>
    </div>

    {#if ["color", "temperature", "brightness", "scene"].includes(kind)}
      <div class="sdpi-item">
        <label class="sdpi-item-label" for="mode-select">Mode</label>
        <select
          class="sdpi-item-value"
          id="mode-select"
          value={settings.mode || "fixed"}
          onchange={(event) => update("mode", event.currentTarget.value)}
        >
          <option value="fixed">Fixed</option>
          <option value="cycle">Cycle</option>
        </select>
      </div>
    {/if}

    {#if kind === "color"}
      {#if settings.mode === "cycle"}
        <div class="sdpi-item">
          <label class="sdpi-item-label" for="cycle-0">Colors</label>
          <div class="sdpi-item-value swatch-list" id="cycle-0">
            {#each settings.colors as color, index (index)}
              <span class="swatch">
                <input
                  type="color"
                  value={color}
                  oninput={(event) => setColorAt(index, event.currentTarget.value)}
                />
                <button
                  type="button"
                  title="Remove"
                  onclick={() => removeColorAt(index)}
                  disabled={settings.colors.length <= 2}>−</button
                >
              </span>
            {/each}
          </div>
        </div>
        <div class="sdpi-row">
          <button class="sdpi-button" onclick={addColor} disabled={settings.colors.length >= 10}>
            + Add color
          </button>
        </div>
      {:else}
        <div class="sdpi-item">
          <label class="sdpi-item-label" for="color-input">Color</label>
          <input
            class="sdpi-item-value"
            id="color-input"
            type="color"
            value={settings.color}
            oninput={(event) => update("color", event.currentTarget.value)}
          />
        </div>
      {/if}
    {/if}

    {#if kind === "brightness"}
      {#if settings.mode === "cycle"}
        <div class="sdpi-item">
          <label class="sdpi-item-label" for="bri-list">Levels</label>
          <div class="sdpi-item-value item-list" id="bri-list">
            {#each settings.brightnesses as bri, index (index)}
              <div class="item-step">
                <input
                  type="range"
                  min="1"
                  max="100"
                  value={bri}
                  oninput={(event) => setBrightnessAt(index, Number(event.currentTarget.value))}
                />
                <span class="step-label">{bri}%</span>
                <button
                  type="button"
                  title="Remove"
                  onclick={() => removeBrightnessAt(index)}
                  disabled={settings.brightnesses.length <= 2}>−</button
                >
              </div>
            {/each}
          </div>
        </div>
        <div class="sdpi-row">
          <button class="sdpi-button" onclick={addBrightness} disabled={settings.brightnesses.length >= 10}>
            + Add level
          </button>
        </div>
      {:else}
        <div class="sdpi-item">
          <label class="sdpi-item-label" for="brightness-input">Brightness ({settings.brightness}%)</label>
          <input
            class="sdpi-item-value"
            id="brightness-input"
            type="range"
            min="1"
            max="100"
            value={settings.brightness}
            oninput={(event) => update("brightness", Number(event.currentTarget.value))}
          />
        </div>
      {/if}
    {/if}

    {#if kind === "temperature"}
      {#if settings.mode === "cycle"}
        <div class="sdpi-item">
          <label class="sdpi-item-label" for="temp-list">Presets</label>
          <div class="sdpi-item-value item-list" id="temp-list">
            {#each settings.temperatures as temp, index (index)}
              <div class="item-step">
                <input
                  type="range"
                  min="1"
                  max="100"
                  value={temp}
                  oninput={(event) => setTemperatureAt(index, Number(event.currentTarget.value))}
                />
                <span class="step-label">{warmthToKelvin(temp)}K</span>
                <button
                  type="button"
                  title="Remove"
                  onclick={() => removeTemperatureAt(index)}
                  disabled={settings.temperatures.length <= 2}>−</button
                >
              </div>
            {/each}
          </div>
        </div>
        <div class="sdpi-row">
          <button class="sdpi-button" onclick={addTemperature} disabled={settings.temperatures.length >= 10}>
            + Add preset
          </button>
        </div>
      {:else}
        <div class="sdpi-item">
          <label class="sdpi-item-label" for="temperature-input">Temperature ({warmthToKelvin(settings.temperature)}K)</label>
          <input
            class="sdpi-item-value"
            id="temperature-input"
            type="range"
            min="1"
            max="100"
            value={settings.temperature}
            oninput={(event) => update("temperature", Number(event.currentTarget.value))}
          />
        </div>
      {/if}
    {/if}

    {#if kind === "scene"}
      {#if settings.mode === "cycle"}
        <div class="sdpi-item">
          <label class="sdpi-item-label" for="scene-list">Scenes</label>
          <div class="sdpi-item-value item-list" id="scene-list">
            {#each settings.scenes as scId, index (index)}
              <div class="item-step">
                <select
                  class="step-select"
                  value={scId}
                  onchange={(event) => setSceneAt(index, event.currentTarget.value)}
                >
                  <option value="">— select —</option>
                  {#each groupScenes as scene (scene.id)}
                    <option value={scene.id}>{scene.name}</option>
                  {/each}
                </select>
                <button
                  type="button"
                  title="Remove"
                  onclick={() => removeSceneAt(index)}
                  disabled={settings.scenes.length <= 2}>−</button
                >
              </div>
            {/each}
          </div>
        </div>
        <div class="sdpi-row">
          <button class="sdpi-button" onclick={addScene} disabled={settings.scenes.length >= 10}>
            + Add scene
          </button>
        </div>
      {:else}
        <div class="sdpi-item">
          <label class="sdpi-item-label" for="scene-select">Scene</label>
          <select
            class="sdpi-item-value"
            id="scene-select"
            value={settings.scene}
            onchange={(event) => update("scene", event.currentTarget.value)}
          >
            <option value="">— select —</option>
            {#each groupScenes as scene (scene.id)}
              <option value={scene.id}>{scene.name}</option>
            {/each}
          </select>
        </div>
      {/if}
      {#if !targetValue.startsWith("g-")}
        <p class="sdpi-note">Scenes apply to a group. Select a group above.</p>
      {/if}
    {/if}

    {#if ["brightness", "temperature"].includes(kind)}
      <div class="sdpi-item">
        <label class="sdpi-item-label" for="scale-ticks">Dial step</label>
        <select
          class="sdpi-item-value"
          id="scale-ticks"
          value={String(settings.scale_ticks)}
          onchange={(event) => update("scale_ticks", Number(event.currentTarget.value))}
        >
          {#each [1, 2, 3, 4, 5, 10] as step (step)}
            <option value={String(step)}>{step}</option>
          {/each}
        </select>
      </div>
    {/if}

    {#if !settings.target && needsTarget}
      <p class="sdpi-note">Pick a light or group to control.</p>
    {/if}
  {/if}
</main>
