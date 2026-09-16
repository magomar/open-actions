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
    color: "#ffcc66",
    colors: ["#ff0000", "#00ff00", "#0000ff"],
    brightness: 100,
    scale_ticks: 1,
    temperature: 366,
    brightness_rel: 10,
    scene: "",
  };

  type Bridge = { ip: string; username: string };
  type Group = { id: string; name: string };
  type Light = { id: string; name: string };

  // The action kind is the last segment of the manifest UUID.
  const kind = $derived(($actionInfo?.action ?? "").split(".").at(-1) ?? "");

  let discovered = $state<{ id: string; internalipaddress: string }[]>([]);
  let groups = $state<Group[]>([]);
  let lights = $state<Light[]>([]);
  let scenes = $state<{ id: string; name: string; group: string }[]>([]);
  let status = $state("");
  let statusIsError = $state(false);
  let busy = $state(false);
  let manualIp = $state("");

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

  eventTarget.addEventListener("bridges" as any, (event: any) => {
    discovered = event.detail?.payload?.bridges ?? [];
    busy = false;
    say(
      discovered.length
        ? `Found ${discovered.length} bridge(s). Select one, then press its link button and Pair.`
        : "No bridges discovered. Enter the IP address manually.",
      !discovered.length,
    );
  });

  eventTarget.addEventListener("paired" as any, (event: any) => {
    const { ip, id, username } = event.detail?.payload ?? {};
    busy = false;
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

  eventTarget.addEventListener("pairError" as any, (event: any) => {
    busy = false;
    say(event.detail?.payload?.message ?? "Pairing failed.", true);
  });

  eventTarget.addEventListener("targets" as any, (event: any) => {
    const targets = event.detail?.payload?.targets ?? {};
    groups = targets.groups ?? [];
    lights = targets.lights ?? [];
    scenes = targets.scenes ?? [];
    busy = false;
    say(`${groups.length} room(s), ${lights.length} light(s), ${scenes.length} scene(s).`);
  });

  eventTarget.addEventListener("targetError" as any, (event: any) => {
    busy = false;
    say(event.detail?.payload?.message ?? "Could not reach the bridge.", true);
  });

  function discover() {
    busy = true;
    say("Searching for bridges…");
    sendToPlugin({ event: "discover" });
  }

  function pair() {
    const ip = manualIp.trim() || discovered[0]?.internalipaddress || activeBridge?.ip;
    if (!ip) {
      say("Enter the bridge IP address first.", true);
      return;
    }
    busy = true;
    say("Press the link button on the bridge, then wait…");
    sendToPlugin({ event: "pair", ip });
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

  const needsTarget = $derived(
    ["power", "color", "cycle", "brightness", "brightness-rel", "temperature", "scene"].includes(kind),
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
    <div class="sdpi-row">
      <button class="sdpi-button" onclick={discover} disabled={busy}>Discover</button>
      <button class="sdpi-button" onclick={pair} disabled={busy}>Pair</button>
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

    {#if kind === "color"}
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

    {#if kind === "cycle"}
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
    {/if}

    {#if kind === "brightness"}
      <div class="sdpi-item">
        <label class="sdpi-item-label" for="brightness-input">Brightness</label>
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

    {#if kind === "brightness-rel"}
      <div class="sdpi-item">
        <label class="sdpi-item-label" for="brightness-rel-input">Steps</label>
        <input
          class="sdpi-item-value"
          id="brightness-rel-input"
          type="range"
          min="-50"
          max="50"
          value={settings.brightness_rel}
          oninput={(event) => update("brightness_rel", Number(event.currentTarget.value))}
        />
      </div>
    {/if}

    {#if kind === "temperature"}
      <div class="sdpi-item">
        <label class="sdpi-item-label" for="temperature-input">Temperature</label>
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

    {#if kind === "scene"}
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
