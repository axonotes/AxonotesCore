<script lang="ts">
  import "./layout.css";
  import {ModeWatcher} from "mode-watcher";
  import LightSwitch from "$lib/components/LightSwitch.svelte";

  let {children} = $props();
</script>

<ModeWatcher />
<div class="absolute flex w-full justify-end p-2">
  <LightSwitch />
</div>
{@render children()}
