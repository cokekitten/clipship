<script lang="ts">
  import type { Config } from "../lib/types";
  import { open } from "@tauri-apps/plugin-dialog";
  import * as Card from "$lib/components/ui/card";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Button } from "$lib/components/ui/button";

  interface SshDraft {
    host: string;
    port: number;
    username: string;
    private_key_path: string;
  }

  let { cfg, onSave }: { cfg: Config; onSave: (draft: SshDraft) => void } = $props();

  let draft: SshDraft = $state({
    host: "",
    port: 22,
    username: "",
    private_key_path: "",
  });

  // Sync draft when cfg changes from the outside (e.g. after loadConfig).
  $effect(() => {
    draft.host = cfg.host;
    draft.port = cfg.port;
    draft.username = cfg.username;
    draft.private_key_path = cfg.private_key_path;
  });

  async function pickKey() {
    const picked = await open({
      multiple: false,
      directory: false,
      title: "Select SSH private key",
    });
    if (typeof picked === "string") draft.private_key_path = picked;
  }

  function handleSave() {
    onSave({ ...draft });
  }
</script>

<Card.Root>
  <Card.Header>
    <Card.Title>SSH</Card.Title>
  </Card.Header>
  <Card.Content class="grid gap-4">
    <div class="grid gap-1.5">
      <Label for="ssh-host">Host</Label>
      <Input id="ssh-host" bind:value={draft.host} placeholder="example.com or ::1" />
    </div>
    <div class="grid grid-cols-2 gap-4">
      <div class="grid gap-1.5">
        <Label for="ssh-port">Port</Label>
        <Input id="ssh-port" type="number" bind:value={draft.port} min="1" max="65535" />
      </div>
      <div class="grid gap-1.5">
        <Label for="ssh-user">Username</Label>
        <Input id="ssh-user" bind:value={draft.username} />
      </div>
    </div>
    <div class="grid gap-1.5">
      <Label for="ssh-key">Private key</Label>
      <div class="flex gap-2">
        <Input id="ssh-key" class="flex-1" bind:value={draft.private_key_path} readonly />
        <Button variant="secondary" type="button" onclick={pickKey}>Browse…</Button>
      </div>
    </div>
    {#if cfg.remote_dir}
      <div class="text-xs text-muted-foreground">
        Remote directory: <span class="font-mono">{cfg.remote_dir}</span>
      </div>
    {/if}
    <Button onclick={handleSave}>Save SSH settings</Button>
  </Card.Content>
</Card.Root>
