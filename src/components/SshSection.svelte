<script lang="ts">
  import type { Config } from "../lib/types";
  import { open } from "@tauri-apps/plugin-dialog";
  import * as Card from "$lib/components/ui/card";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Button } from "$lib/components/ui/button";

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

<Card.Root>
  <Card.Header>
    <Card.Title>SSH</Card.Title>
  </Card.Header>
  <Card.Content class="grid gap-4">
    <div class="grid gap-1.5">
      <Label for="ssh-host">Host</Label>
      <Input id="ssh-host" bind:value={cfg.host} placeholder="example.com or ::1" />
    </div>
    <div class="grid grid-cols-2 gap-4">
      <div class="grid gap-1.5">
        <Label for="ssh-port">Port</Label>
        <Input id="ssh-port" type="number" bind:value={cfg.port} min="1" max="65535" />
      </div>
      <div class="grid gap-1.5">
        <Label for="ssh-user">Username</Label>
        <Input id="ssh-user" bind:value={cfg.username} />
      </div>
    </div>
    <div class="grid gap-1.5">
      <Label for="ssh-key">Private key</Label>
      <div class="flex gap-2">
        <Input id="ssh-key" class="flex-1" bind:value={cfg.private_key_path} readonly />
        <Button variant="secondary" type="button" onclick={pickKey}>Browse…</Button>
      </div>
    </div>
  </Card.Content>
</Card.Root>
