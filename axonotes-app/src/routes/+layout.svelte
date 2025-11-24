<script lang="ts">
  import "./layout.css";
  import {ModeWatcher} from "mode-watcher";
  import LightSwitch from "$lib/components/LightSwitch.svelte";
  import {onMount} from "svelte";
  import {activeProfile, initAuth} from "$lib/stores/authStore";
  import ProfileSwitcher from "$lib/components/ProfileSwitcher.svelte";

  let {children} = $props();

  let isInitialized = $state(false);

  onMount(async () => {
    await initAuth();
    isInitialized = true;

    // Check if we need to redirect to login
    if (!$activeProfile && window.location.pathname !== "/login") {
      window.location.href = "/login";
    }
  });
</script>

<ModeWatcher />
<div class="absolute flex w-full justify-end p-3.5">
  <LightSwitch />
</div>

{#if isInitialized}
  {#if $activeProfile}
    <!-- App layout with profile switcher -->
    <div class="bg-background min-h-screen">
      <!-- Header -->
      <header class="border-b">
        <div class="container flex h-16 items-center justify-between px-4">
          <div class="flex items-center gap-2">
            <h1 class="text-xl font-bold">My App</h1>
          </div>

          <div class="w-64">
            <ProfileSwitcher />
          </div>
        </div>
      </header>

      <!-- Main content -->
      <main class="container px-4 py-6">
        {@render children()}
      </main>
    </div>
  {:else}
    <!-- Show login page if no active profile -->
    {@render children()}
  {/if}
{:else}
  <!-- Loading state -->
  <div class="flex min-h-screen items-center justify-center">
    <div class="text-center">
      <div
        class="inline-block h-8 w-8 animate-spin rounded-full border-4 border-solid border-current border-r-transparent align-[-0.125em] motion-reduce:animate-[spin_1.5s_linear_infinite]"
      ></div>
      <p class="text-muted-foreground mt-4">Loading...</p>
    </div>
  </div>
{/if}
