<script lang="ts">
  import "./layout.css";
  import {ModeWatcher} from "mode-watcher";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import {onMount} from "svelte";
  import {goto} from "$app/navigation";
  import {isAuthenticated, isUnlocked} from "$lib/stores/app";
  import {get} from "svelte/store";

  let {children} = $props();

  $effect(() => {
    const unlocked = $isUnlocked;
    const authenticated = $isAuthenticated;
    if (!unlocked) {
      goto('/unlock');
    } else if (!authenticated) {
      goto('/login');
    } else {
      goto('/app');
    }
  })

  // onMount(async () => {
  //   try {
  //     // TODO: rework this bullshit routing logic
  //     if (!get(isUnlocked)) {
  //       await goto('/unlock');
  //     } else {
  //       await goto('/login');
  //     }
  //   } catch (error) {
  //     console.error('Error checking database status:', error);
  //     // In case of error, assume locked for security
  //     await goto('/unlock');
  //   }
  // });
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
