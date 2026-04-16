<script lang="ts">
  import type { Config } from "../lib/types";
  import * as Card from "$lib/components/ui/card";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";

  let { cfg = $bindable() }: { cfg: Config } = $props();
</script>

<Card.Root>
  <Card.Header>
    <Card.Title>Destination</Card.Title>
  </Card.Header>
  <Card.Content class="grid gap-4">
    <div class="grid gap-1.5">
      <Label for="remote-dir">Remote directory</Label>
      <Input id="remote-dir" bind:value={cfg.remote_dir} placeholder="/var/www/uploads" />
    </div>
  </Card.Content>
</Card.Root>
