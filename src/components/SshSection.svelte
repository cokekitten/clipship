<script lang="ts">
  import type { Config } from "../lib/types";
  import { open } from "@tauri-apps/plugin-dialog";
  let { cfg = $bindable() }: { cfg: Config } = $props();

  async function pickKey() {
    const picked = await open({
      multiple: false,
      directory: false,
      title: "Select SSH private key",
    });
    if (typeof picked === "string") cfg.private_key_path = picked;
  }
</script>

<fieldset>
  <legend>SSH</legend>
  <label>Host <input bind:value={cfg.host} placeholder="example.com or ::1" /></label>
  <label>Port <input type="number" bind:value={cfg.port} min="1" max="65535" /></label>
  <label>Username <input bind:value={cfg.username} /></label>
  <label>Private key
    <input bind:value={cfg.private_key_path} readonly />
    <button type="button" onclick={pickKey}>Browse…</button>
  </label>
</fieldset>
