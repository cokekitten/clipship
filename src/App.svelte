<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { Config, Status } from "./lib/types";
  import { loadConfig, saveGeneralConfig, saveSshConfig, getAutostart, setAutostart, defaultPrivateKey } from "./lib/bridge";
  import SshSection from "./components/SshSection.svelte";
  import ShortcutSection from "./components/ShortcutSection.svelte";
  import StatusArea from "./components/StatusArea.svelte";
  import * as Card from "$lib/components/ui/card";
  import { Switch } from "$lib/components/ui/switch";
  import { Label } from "$lib/components/ui/label";
  import { Button } from "$lib/components/ui/button";

  let cfg: Config = $state({
    version: 1,
    mode: "local",
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
  let version: string = $state("");
  // Prevents auto-save from firing before the initial config load completes.
  let ready = $state(false);
  let autoSaveTimer: ReturnType<typeof setTimeout> | undefined;
  // Prevents auto-save from racing with an explicit SSH save.
  let savingSsh = $state(false);

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
    try {
      version = await getVersion();
    } catch (_) {}
    ready = true;
  });

  // Auto-save general (non-SSH) config fields immediately.
  $effect(() => {
    const _snap = JSON.stringify(cfg); // track all cfg fields reactively
    if (!ready || savingSsh) return;
    clearTimeout(autoSaveTimer);
    autoSaveTimer = setTimeout(async () => {
      try {
        await saveGeneralConfig(cfg);
      } catch (e) {
        status = { kind: "error", message: "Auto-save failed", detail: String(e) };
      }
    }, 300);
  });

  async function handleSshSave(draft: { host: string; port: number; username: string; private_key_path: string }) {
    savingSsh = true;
    try {
      cfg.host = draft.host;
      cfg.port = draft.port;
      cfg.username = draft.username;
      cfg.private_key_path = draft.private_key_path;

      const result = await saveSshConfig(cfg);
      status = result.warnings.length
        ? { kind: "ok", message: "Connected and saved with warnings.", detail: result.warnings.join("\n") }
        : { kind: "ok", message: "Connected and saved." };
      cfg = await loadConfig();
    } catch (e) {
      status = { kind: "error", message: "Save failed", detail: String(e) };
    } finally {
      savingSsh = false;
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
    <div class="flex items-center gap-2">
      <h1 class="text-xl font-semibold leading-none">Clipship</h1>
      {#if version}
        <span class="text-xs text-muted-foreground pt-0.5">v{version}</span>
      {/if}
    </div>
    <a
      href="https://github.com/cokekitten/clipship"
      onclick={(e) => { e.preventDefault(); openUrl("https://github.com/cokekitten/clipship"); }}
      class="text-muted-foreground hover:text-foreground transition-colors"
      aria-label="GitHub"
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="size-5">
        <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4"/>
        <path d="M9 18c-4.51 2-5-2-7-2"/>
      </svg>
    </a>
  </div>
  <div class="flex gap-1 rounded-md border p-1 w-fit" role="group" aria-label="Upload mode">
    <Button
      variant={cfg.mode === "local" ? "default" : "ghost"}
      aria-pressed={cfg.mode === "local"}
      onclick={() => cfg.mode = "local"}
      size="sm">Local</Button>
    <Button
      variant={cfg.mode === "ssh" ? "default" : "ghost"}
      aria-pressed={cfg.mode === "ssh"}
      onclick={() => cfg.mode = "ssh"}
      size="sm">SSH</Button>
  </div>
  <div
    class={`flex flex-col gap-4 ${cfg.mode === "local" ? "pointer-events-none opacity-50" : ""}`}
    inert={cfg.mode === "local" || undefined}
  >
    <SshSection {cfg} onSave={handleSshSave} />
  </div>
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
