<script lang="ts">
  import type { Config } from "../lib/types";
  let { cfg = $bindable() }: { cfg: Config } = $props();
</script>

<fieldset>
  <legend>Destination</legend>
  <label>Remote directory
    <input bind:value={cfg.remote_dir} placeholder="/home/ubuntu/uploads" />
  </label>
  <p class="hint">ASCII only. Must start with '/'. No spaces, quotes, or shell metacharacters.</p>
</fieldset>
