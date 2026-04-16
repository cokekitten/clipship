<script lang="ts">
  import type { Config } from "../lib/types";
  import ShortcutRecorder from "./ShortcutRecorder.svelte";
  import * as Card from "$lib/components/ui/card";
  import { Label } from "$lib/components/ui/label";
  import { Switch } from "$lib/components/ui/switch";

  let { cfg = $bindable() }: { cfg: Config } = $props();

  function onAccel(v: string) {
    cfg.shortcut = v;
  }

  function onDoubleTapChange(v: boolean) {
    cfg.shortcut_double_tap = v;
  }
</script>

<Card.Root>
  <Card.Header>
    <Card.Title>Shortcut</Card.Title>
  </Card.Header>
  <Card.Content class="grid gap-4">
    <div class="grid gap-1.5">
      <Label>Global shortcut</Label>
      <ShortcutRecorder value={cfg.shortcut} onChange={onAccel} />
    </div>
    <div class="flex items-center justify-between">
      <Label class="flex flex-col gap-1 items-start">
        <span>Require double-tap to trigger</span>
        <span class="text-xs font-normal text-muted-foreground">
          Press the shortcut twice within 400&nbsp;ms to upload.
        </span>
      </Label>
      <Switch
        checked={cfg.shortcut_double_tap}
        onCheckedChange={onDoubleTapChange}
      />
    </div>
  </Card.Content>
</Card.Root>
