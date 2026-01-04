<script lang="ts">
  import "./layout.css";
  import {ModeWatcher} from "mode-watcher";
  import TitleBar from "$lib/components/TitleBar.svelte";

  let {children} = $props();
</script>

<ModeWatcher />

<!-- TitleBar at the very top -->
<div class="flex h-screen flex-col overflow-hidden">
  <TitleBar />

  <!-- Content area -->
  <div class="flex-1 overflow-auto">
    {@render children()}
  </div>
</div>
