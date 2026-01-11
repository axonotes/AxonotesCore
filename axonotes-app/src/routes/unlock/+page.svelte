<script lang="ts">
  import {onMount} from "svelte";
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {app, databaseMode} from "$lib/stores/app";
  import {Button} from "$lib/components/ui/button";
  import PasswordInput from "$lib/components/auth/PasswordInput.svelte";
  import PinInput from "$lib/components/auth/PinInput.svelte";
  import {Loader2, Lock} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  // Local display mode - initialized from store, can be toggled by user
  // This allows users to switch input method without changing the actual stored preference
  let displayMode = $state<"pin" | "pass">("pass");
  let password = $state("");
  let error = $state("");
  let isUnlocking = $state(false);
  let pinInput: {focus: () => void} | undefined = $state(undefined);

  // Initialize display mode from store when it's available
  $effect(() => {
    if ($databaseMode && $databaseMode !== "none") {
      displayMode = $databaseMode;
    }
  });

  // Focus input on any keypress when not already focused
  function handleWindowKeydown(event: KeyboardEvent) {
    // Ignore if already focused on an input
    if (document.activeElement?.tagName === "INPUT") return;
    // Ignore modifier keys and special keys
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.key.length !== 1 && event.key !== "Backspace") return;

    // Focus the appropriate input
    if (displayMode === "pin") {
      pinInput?.focus();
    }
    // For password mode, PasswordInput handles its own focus
  }

  onMount(() => {
    window.addEventListener("keydown", handleWindowKeydown);
    return () => window.removeEventListener("keydown", handleWindowKeydown);
  });

  function switchMode() {
    // Toggle between pin and pass for display only
    displayMode = displayMode === "pass" ? "pin" : "pass";
    password = "";
    error = "";
  }

  // Helper to add timeout to a promise
  function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
    return Promise.race([
      promise,
      new Promise<T>((_, reject) =>
        setTimeout(() => reject(new Error("Timeout")), ms)
      ),
    ]);
  }

  async function handleUnlock() {
    error = "";

    if (!password) {
      error = m.auth_unlock_error_incorrect();
      return;
    }

    if (displayMode === "pin" && password.length !== 6) {
      error = m.auth_unlock_error_incorrect();
      return;
    }

    isUnlocking = true;

    try {
      // Add 2 second timeout to prevent long waits on wrong password
      await withTimeout(app.unlockDatabase(password), 2000);
      // Let layout handle redirect based on state
      goto(resolve("/"), {replaceState: true});
    } catch (err) {
      console.error("Unlock error:", err);
      error = m.auth_unlock_error_incorrect();
      password = "";
    } finally {
      isUnlocking = false;
    }
  }
</script>

<div
  class="bg-background flex h-full w-full flex-col items-center justify-center p-6"
>
  <div class="w-full max-w-sm space-y-8">
    <!-- Header -->
    <div class="space-y-4 text-center">
      <div
        class="bg-muted mx-auto flex h-12 w-12 items-center justify-center rounded-full"
      >
        <Lock class="text-muted-foreground h-5 w-5" />
      </div>
      <h1 class="text-3xl font-semibold tracking-tight">
        {m.auth_unlock_title()}
      </h1>
    </div>

    <!-- Input -->
    <form
      class="space-y-5"
      onsubmit={(e) => {
        e.preventDefault();
        handleUnlock();
      }}
    >
      {#if displayMode === "pin"}
        <PinInput
          id="pin"
          bind:this={pinInput}
          bind:value={password}
          disabled={isUnlocking}
          autofocus
          onenter={handleUnlock}
        />
      {:else}
        <PasswordInput
          id="password"
          placeholder={m.auth_unlock_password_placeholder()}
          bind:value={password}
          disabled={isUnlocking}
          autofocus
        />
      {/if}

      {#if error}
        <p class="text-destructive text-center text-sm">{error}</p>
      {/if}

      <Button class="h-10 w-full" disabled={isUnlocking} type="submit">
        {#if isUnlocking}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
          {m.auth_unlock_button_loading()}
        {:else}
          {m.auth_unlock_button_primary()}
        {/if}
      </Button>
    </form>

    <!-- Very subtle mode switch - only for edge cases -->
    <button
      class="text-muted-foreground/50 hover:text-muted-foreground mx-auto block text-xs transition-colors disabled:pointer-events-none"
      disabled={isUnlocking}
      onclick={switchMode}
      type="button"
    >
      {displayMode === "pin"
        ? m.auth_unlock_link_switch_password()
        : m.auth_unlock_link_switch_pin()}
    </button>
  </div>
</div>
