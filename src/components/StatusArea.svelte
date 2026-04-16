<script lang="ts">
  import type { Status } from "../lib/types";
  let { status }: { status: Status } = $props();
</script>

{#if status.kind !== "idle" || status.message}
  <div
    class={[
      "rounded-md border p-3 text-sm",
      status.kind === "error" ? "border-destructive/30 bg-destructive/5 text-destructive" : "",
      status.kind === "ok" ? "border-emerald-200 bg-emerald-50 text-emerald-800" : "",
      status.kind === "idle" ? "border-border bg-muted text-muted-foreground" : "",
    ].join(" ")}
  >
    <div class="font-medium">{status.message}</div>
    {#if status.detail}
      <pre class="mt-2 whitespace-pre-wrap text-xs">{status.detail}</pre>
    {/if}
  </div>
{/if}
