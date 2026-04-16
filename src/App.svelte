<script lang="ts">
  import { onMount } from "svelte";
  import type { Config, Status } from "./lib/types";
  import { loadConfig, saveConfig, testConnection } from "./lib/bridge";
  import SshSection from "./components/SshSection.svelte";
  import DestinationSection from "./components/DestinationSection.svelte";
  import ShortcutSection from "./components/ShortcutSection.svelte";
  import StatusArea from "./components/StatusArea.svelte";

  let cfg: Config = $state({
    version: 1,
    host: "",
    port: 22,
    username: "",
    private_key_path: "",
    remote_dir: "",
    shortcut: "CmdOrCtrl+Shift+U",
  });

  let status: Status = $state({ kind: "idle", message: "" });

  onMount(async () => {
    try {
      cfg = await loadConfig();
    } catch (e) {
      status = { kind: "error", message: "Failed to load configuration", detail: String(e) };
    }
  });

  async function onSave() {
    try {
      const result = await saveConfig(cfg);
      status = result.warnings.length
        ? { kind: "ok", message: "Saved with warnings.", detail: result.warnings.join("\n") }
        : { kind: "ok", message: "Saved." };
    } catch (e) {
      status = { kind: "error", message: "Save failed", detail: String(e) };
    }
  }

  async function onTest() {
    status = { kind: "idle", message: "Testing…" };
    try {
      await testConnection(cfg);
      status = { kind: "ok", message: "Connection OK." };
    } catch (e) {
      status = { kind: "error", message: "Test failed", detail: String(e) };
    }
  }
</script>

<main>
  <h1>Clipship</h1>
  <SshSection bind:cfg />
  <DestinationSection bind:cfg />
  <ShortcutSection bind:cfg />
  <div class="actions">
    <button onclick={onSave}>Save</button>
    <button onclick={onTest}>Test connection</button>
  </div>
  <StatusArea {status} />
</main>

<style>
  main { padding: 1rem; font-family: system-ui; }
  fieldset { margin-bottom: 1rem; }
  label { display: block; margin: 0.25rem 0; }
  .hint { font-size: 0.85rem; color: #666; }
  .status.error pre { background: #fee; padding: 0.5rem; white-space: pre-wrap; }
  .status.ok { color: #060; }
</style>
