<script lang="ts">
  import { actionSettings } from "@openaction/svelte-pi";

  const defaults = {
    handy_binary_path: "",
  };

  function update(key: keyof typeof defaults, value: unknown) {
    actionSettings.update((saved) => ({ ...defaults, ...saved, [key]: value }));
  }
</script>

<main class="sdpi-wrapper">
  <div class="sdpi-item">
    <label class="sdpi-item-label" for="handy-binary-path">Handy binary path</label>
    <input class="sdpi-item-value" id="handy-binary-path" type="text" placeholder="handy" value={$actionSettings.handy_binary_path ?? ""} oninput={(event) => update("handy_binary_path", event.currentTarget.value)} />
  </div>
  <p class="sdpi-note">Leave empty to auto-detect (<code>handy</code> on PATH, or the macOS app bundle). The button toggles Handy transcription.</p>
</main>
