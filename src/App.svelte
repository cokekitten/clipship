<script lang="ts">
  import { onMount } from "svelte";
  import type { Config, Status } from "./lib/types";
  import { loadConfig, saveConfig, testConnection, getAutostart, setAutostart, defaultPrivateKey } from "./lib/bridge";
  import SshSection from "./components/SshSection.svelte";
  import DestinationSection from "./components/DestinationSection.svelte";
  import ShortcutSection from "./components/ShortcutSection.svelte";
  import StatusArea from "./components/StatusArea.svelte";
  import * as Card from "$lib/components/ui/card";
  import { Switch } from "$lib/components/ui/switch";
  import { Label } from "$lib/components/ui/label";
  import { Button } from "$lib/components/ui/button";

  let cfg: Config = $state({
    version: 1,
    mode: "ssh",
    host: "",
    port: 22,
    username: "",
    private_key_path: "",
    remote_dir: "",
    shortcut: "CmdOrCtrl+Shift+U",
    shortcut_double_tap: false,
    auto_cleanup: false,
  });

  let status: Status = $state({ kind: "idle", message: "" });
  let autostart: boolean = $state(false);

  onMount(async () => {
    try {
      cfg = await loadConfig();
    } catch (e) {
      status = { kind: "error", message: "Failed to load configuration", detail: String(e) };
    }
    if (!cfg.private_key_path) {
      try {
        const k = await defaultPrivateKey();
        if (k) cfg.private_key_path = k;
      } catch (_) {}
    }
    try {
      autostart = await getAutostart();
    } catch (_) {}
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

  async function onAutostartChange(v: boolean) {
    autostart = v;
    try {
      await setAutostart(autostart);
    } catch (e) {
      status = { kind: "error", message: "Failed to update autostart", detail: String(e) };
      autostart = !autostart;
    }
  }
</script>

<main class="mx-auto flex max-w-2xl flex-col gap-4 p-6">
  <div class="flex items-center justify-between">
    <h1 class="text-xl font-semibold leading-none">Clipship</h1>
    <Button onclick={onSave}>Save</Button>
  </div>
  <div class="flex gap-1 rounded-md border p-1 w-fit" role="group" aria-label="Upload mode">
    <Button
      variant={cfg.mode === "ssh" ? "default" : "ghost"}
      aria-pressed={cfg.mode === "ssh"}
      onclick={() => cfg.mode = "ssh"}
      size="sm">SSH</Button>
    <Button
      variant={cfg.mode === "local" ? "default" : "ghost"}
      aria-pressed={cfg.mode === "local"}
      onclick={() => cfg.mode = "local"}
      size="sm">Local</Button>
  </div>
  <div
    class={`flex flex-col gap-4 ${cfg.mode === "local" ? "pointer-events-none opacity-50" : ""}`}
    inert={cfg.mode === "local" || undefined}
  >
    <SshSection bind:cfg />
    <DestinationSection bind:cfg />
  </div>
  {#if cfg.mode === "ssh"}
    <Button variant="secondary" onclick={onTest}>Test connection</Button>
  {/if}
  <ShortcutSection bind:cfg />
  <Card.Root>
    <Card.Header>
      <Card.Title>System</Card.Title>
    </Card.Header>
    <Card.Content class="flex items-center justify-between">
      <Label class="flex flex-col gap-1 items-start">
        <span>Launch at login</span>
        <span class="text-xs font-normal text-muted-foreground">
          Start Clipship automatically when you sign in.
        </span>
      </Label>
      <Switch checked={autostart} onCheckedChange={onAutostartChange} />
    </Card.Content>
    <Card.Content class="flex items-center justify-between">
      <div class="flex flex-col gap-1 items-start">
        <span class="text-sm font-medium">Auto-cleanup</span>
        <span class="text-xs text-muted-foreground">
          Delete files older than 7 days every hour. Remote cleanup requires SSH config to be complete.
        </span>
      </div>
      <Switch checked={cfg.auto_cleanup} onCheckedChange={(v) => cfg.auto_cleanup = v} />
    </Card.Content>
  </Card.Root>
  <StatusArea {status} />
</main>
