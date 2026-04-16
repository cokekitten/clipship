<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { eventToAccelerator } from "../lib/shortcut/map";

  let {
    value,
    onChange,
  }: { value: string; onChange: (v: string) => void } = $props();

  let recording = $state(false);

  function start() {
    recording = true;
  }

  function stop() {
    recording = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();

    if (e.key === "Escape" && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) {
      stop();
      return;
    }

    const accel = eventToAccelerator(e);
    if (!accel) {
      return; // wait for a valid combo — modifier-only / no-modifier events ignored
    }
    onChange(accel);
    stop();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex items-center gap-2">
  {#if recording}
    <div
      class="inline-flex h-9 items-center rounded-md border border-amber-400 bg-amber-50 px-3 text-sm text-amber-900"
    >
      Press shortcut… <span class="ml-2 text-xs text-amber-700">(Esc to cancel)</span>
    </div>
    <Button variant="secondary" type="button" onclick={stop}>Cancel</Button>
  {:else}
    <code class="inline-flex h-9 items-center rounded-md border bg-muted px-3 text-sm">
      {value || "(unset)"}
    </code>
    <Button variant="secondary" type="button" onclick={start}>Change</Button>
  {/if}
</div>
