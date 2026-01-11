<script lang="ts">
  import "./layout.css";
  import {ModeWatcher} from "mode-watcher";
  import LightSwitch from "$lib/components/LightSwitch.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import {Spinner} from "$lib/components/ui/spinner";
  import {onMount} from "svelte";
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {
    app,
    isInitialized,
    activeProfile,
    databaseUnlocked,
    needsUnlock,
    needsSetup,
    needsSync,
    isReady,
    stdbUserExists,
    keysNeedSync,
    isSettingUpLocalSecurity,
  } from "$lib/stores/app";
  import * as m from "$lib/paraglide/messages.js";

  let {children} = $props();

  // Auth flow pages that don't need redirects when in their flow
  const authPages = ["/login", "/unlock", "/setup", "/sync"];
  function isAuthPage(path: string): boolean {
    return authPages.some((p) => path === p || path.startsWith(p + "/"));
  }

  // Local security pages - accessible during setup/sync even when isReady
  function isLocalSecurityPage(path: string): boolean {
    return path.startsWith("/setup/local-security");
  }

  onMount(async () => {
    await app.initialize();
    handleRedirects();
  });

  // React to state changes and redirect accordingly
  $effect(() => {
    if ($isInitialized) {
      handleRedirects();
    }
  });

  function handleRedirects() {
    const path = window.location.pathname;

    // Priority 1: Database needs unlock
    if ($needsUnlock) {
      if (path !== "/unlock") {
        goto(resolve("/unlock"), {replaceState: true});
      }
      return;
    }

    // Priority 2: No active profile → login
    if ($databaseUnlocked && !$activeProfile) {
      if (path !== "/login") {
        goto(resolve("/login"), {replaceState: true});
      }
      return;
    }

    // Priority 3: User doesn't exist on STDB → setup wizard
    if ($needsSetup) {
      if (!path.startsWith("/setup")) {
        goto(resolve("/setup/master-password"), {replaceState: true});
      }
      return;
    }

    // Priority 4: Keys need sync → sync page
    if ($needsSync) {
      if (!path.startsWith("/sync")) {
        goto(resolve("/sync"), {replaceState: true});
      }
      return;
    }

    // Priority 5: Allow local security pages during setup flow
    // (User is technically ready but we want them to set local security first)
    if ($isSettingUpLocalSecurity && isLocalSecurityPage(path)) {
      // Don't redirect - let them finish local security setup
      return;
    }

    // Priority 6: Handle null/error states - show error or retry
    // If we have a profile but encryption state is null, something went wrong
    if (
      $activeProfile &&
      ($stdbUserExists === null || ($stdbUserExists && $keysNeedSync === null))
    ) {
      // Error state - encryption check failed. Try to reinitialize.
      console.warn("[Layout] Encryption state is null, attempting re-check...");
      app.checkEncryptionState();
      return;
    }

    // Priority 7: User is ready → redirect away from auth pages
    if ($isReady && isAuthPage(path)) {
      goto(resolve("/app"), {replaceState: true});
      return;
    }
  }
</script>

<ModeWatcher />

<!-- TitleBar at the very top -->
<div class="flex h-screen flex-col overflow-hidden">
  <TitleBar />

  <!-- Content area -->
  <div class="flex-1 overflow-hidden">
    {#if $isInitialized}
      {#if $isReady && !$isSettingUpLocalSecurity}
        <!-- App layout - only when fully ready (logged in, keys synced) -->
        <div class="bg-background relative h-full">
          {@render children()}
        </div>
      {:else}
        <!-- Auth pages: login, unlock, setup, sync -->
        <div class="relative h-full min-h-full">
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
          <Spinner size="lg" />
          <p class="text-muted-foreground mt-4">{m.common_loading()}</p>
        </div>
      </div>
    {/if}
  </div>
</div>
