<script lang="ts">
  import "./layout.css";
  import {ModeWatcher} from "mode-watcher";
  import LightSwitch from "$lib/components/LightSwitch.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
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

<!-- TitleBar at the very top -->
<div class="flex h-screen flex-col overflow-hidden">
  <TitleBar />

  <!-- Content area -->
  <div class="flex-1 overflow-auto">
    {#if isInitialized}
      {#if $activeProfile}
        <!-- App layout with profile switcher -->
        <div class="bg-background min-h-full">
          <!-- Header -->
          <header class="border-b">
            <div class="container flex h-16 items-center justify-between px-4">
              <div class="flex items-center gap-2">
                <h1 class="text-xl font-bold">My App</h1>
              </div>

              <div class="flex items-center gap-3">
                <LightSwitch />
                <div class="w-64">
                  <ProfileSwitcher />
                </div>
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
        <div class="relative min-h-full">
          <div class="absolute top-0 right-0 p-3.5">
            <LightSwitch />
          </div>
          {@render children()}
        </div>
      {/if}
    {:else}
      <!-- Loading state -->
      <div class="flex min-h-full items-center justify-center">
        <div class="text-center">
          <div
            class="inline-block h-8 w-8 animate-spin rounded-full border-4 border-solid border-current border-r-transparent align-[-0.125em] motion-reduce:animate-[spin_1.5s_linear_infinite]"
          ></div>
          <p class="text-muted-foreground mt-4">Loading...</p>
        </div>
      </div>
    {/if}
  </div>
</div>
