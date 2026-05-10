<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  let { currentVersion = "" }: { currentVersion: string } = $props();
  let latestVersion: string = $state("");
  let hasUpdate: boolean = $state(false);

  function compareVersions(a: string, b: string): number {
    const pa = a.replace(/^v/, "").split(".").map(Number);
    const pb = b.replace(/^v/, "").split(".").map(Number);
    for (let i = 0; i < 3; i++) {
      if (pa[i] > pb[i]) return 1;
      if (pa[i] < pb[i]) return -1;
    }
    return 0;
  }

  onMount(async () => {
    if (!currentVersion) return;
    try {
      const resp = await fetch("https://api.github.com/repos/cokekitten/clipship/releases/latest");
      if (!resp.ok) return;
      const data = await resp.json();
      const tag: string = data.tag_name;
      if (tag && compareVersions(tag, currentVersion) > 0) {
        latestVersion = tag;
        hasUpdate = true;
      }
    } catch {
      // Silently ignore network errors
    }
  });
</script>

{#if hasUpdate}
  <a
    href="https://github.com/cokekitten/clipship/releases/latest"
    onclick={(e) => { e.preventDefault(); openUrl("https://github.com/cokekitten/clipship/releases/latest"); }}
    class="inline-flex items-center gap-1 rounded-full bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-500 hover:bg-blue-500/20 transition-colors cursor-pointer no-underline"
    aria-label="Update available: {latestVersion}"
  >
    <span class="inline-block h-1.5 w-1.5 rounded-full bg-blue-500"></span>
    <span>v{latestVersion} available</span>
  </a>
{/if}