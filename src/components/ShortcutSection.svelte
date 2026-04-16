<script lang="ts">
  import type { Config } from "../lib/types";
  let { cfg = $bindable() }: { cfg: Config } = $props();
</script>

<fieldset>
  <legend>Shortcut</legend>
  <label>Global shortcut <input bind:value={cfg.shortcut} placeholder="CmdOrCtrl+Shift+U" /></label>
</fieldset>
