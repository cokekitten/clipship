<script lang="ts">
  import type { Status } from "../lib/types";
  let { status }: { status: Status } = $props();
</script>

<section class={"status " + status.kind}>
  <p>{status.message}</p>
  {#if status.detail}<pre>{status.detail}</pre>{/if}
</section>
