<script lang="ts">
  import type { Status } from "../lib/types";

  let { status }: { status: Status } = $props();

  let visible = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (status.kind !== "idle" || status.message) {
      visible = true;
      clearTimeout(timer);
      if (status.kind === "ok") {
        timer = setTimeout(() => { visible = false; }, 3000);
      }
    } else {
      visible = false;
    }
  });
</script>

{#if visible}
  <div class="fixed top-4 left-1/2 -translate-x-1/2 z-50 min-w-48 max-w-sm pointer-events-none">
    <div
      class={[
        "rounded-md border px-4 py-3 text-sm shadow-md text-center",
        status.kind === "error" ? "border-destructive/30 bg-destructive/5 text-destructive" : "",
        status.kind === "ok" ? "border-emerald-200 bg-emerald-50 text-emerald-800" : "",
        status.kind === "idle" ? "border-border bg-muted text-muted-foreground" : "",
      ].join(" ")}
    >
      <div class="font-medium">{status.message}</div>
      {#if status.detail}
        <pre class="mt-2 whitespace-pre-wrap text-xs text-left">{status.detail}</pre>
      {/if}
    </div>
  </div>
{/if}
