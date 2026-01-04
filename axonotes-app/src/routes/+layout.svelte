<script lang="ts">
  import "./layout.css";
  import {ModeWatcher} from "mode-watcher";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import {onMount} from "svelte";
  import {isDatabaseUnlocked} from "$lib/services/database";
  import {goto} from "$app/navigation";

  let {children} = $props();

  onMount(async () => {
    try {
      const isUnlocked = await isDatabaseUnlocked();
      if (!isUnlocked) {
        await goto('/unlock');
      } else {
        await goto('/login');
      }
    } catch (error) {
      console.error('Error checking database status:', error);
      // In case of error, assume locked for security
      await goto('/unlock');
    }
  });
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
