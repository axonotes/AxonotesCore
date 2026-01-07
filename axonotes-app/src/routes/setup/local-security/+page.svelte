<script lang="ts">
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {onMount} from "svelte";
  import {get} from "svelte/store";
  import {slide} from "svelte/transition";
  import {Button} from "$lib/components/ui/button";
  import {setupStore} from "$lib/stores/setup";
  import {app, isReady, isSettingUpLocalSecurity} from "$lib/stores/app";
  import {
    ShieldCheck,
    Hash,
    KeyRound,
    SkipForward,
    Loader2,
  } from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  type SecurityOption = "pin" | "password" | "skip";

  let selectedOption = $state<SecurityOption | null>(null);
  let error = $state("");
  let isLoading = $state(false);
  let isFromSyncFlow = $state(false);

  onMount(() => {
    const state = get(setupStore);
    const settingUpLocal = get(isSettingUpLocalSecurity);
    const ready = get(isReady);

    // Case 1: Coming from sync/recovery flow (isSettingUpLocalSecurity is true)
    if (settingUpLocal) {
      isFromSyncFlow = true;
      return;
    }

    // Case 2: Coming from new user setup (mnemonic exists in store)
    if (state.mnemonic) {
      isFromSyncFlow = false;
      return;
    }

    // Case 3: Already fully set up - redirect to app
    if (ready) {
      goto(resolve("/app"), {replaceState: true});
      return;
    }

    // Case 4: Invalid access - redirect to start
    goto(resolve("/"), {replaceState: true});
  });

  async function handleContinue() {
    if (!selectedOption) return;

    if (selectedOption === "pin") {
      goto(resolve("/setup/local-security/pin"));
      return;
    }

    if (selectedOption === "password") {
      goto(resolve("/setup/local-security/password"));
      return;
    }

    // Skip - finish setup without encryption
    isLoading = true;
    error = "";

    try {
      setupStore.reset();
      await app.finishLocalSecuritySetup();
      goto(resolve("/app"), {replaceState: true});
    } catch (err) {
      console.error("Failed to complete setup:", err);
      error = String(err);
    } finally {
      isLoading = false;
    }
  }
</script>

<div
  class="bg-background flex min-h-full w-full items-center justify-center p-6"
>
  <div class="w-full max-w-sm space-y-8">
    <!-- Header -->
    <div class="space-y-4 text-center">
      <div
        class="bg-muted mx-auto flex h-12 w-12 items-center justify-center rounded-full"
      >
        <ShieldCheck class="text-muted-foreground h-5 w-5" />
      </div>
      <div class="space-y-2">
        {#if !isFromSyncFlow}
          <p
            class="text-muted-foreground text-xs font-medium tracking-wide uppercase"
          >
            {m.auth_setup_step({current: 3, total: 3})}
          </p>
        {/if}
        <h1 class="text-xl font-semibold tracking-tight">
          {m.auth_setup_security_title()}
        </h1>
      </div>
    </div>

    <!-- Options -->
    <div class="space-y-3">
      <button
        class="hover:bg-accent w-full rounded-lg border p-3 text-left transition-colors {selectedOption ===
        'pin'
          ? 'border-primary bg-accent'
          : ''}"
        onclick={() => (selectedOption = "pin")}
        type="button"
      >
        <div class="flex items-center gap-3">
          <Hash class="text-muted-foreground h-4 w-4" />
          <div>
            <p class="text-sm font-medium">
              {m.auth_setup_security_option_pin()}
            </p>
            <p class="text-muted-foreground text-xs">
              {m.auth_setup_security_option_pin_description()}
            </p>
          </div>
        </div>
      </button>

      <button
        class="hover:bg-accent w-full rounded-lg border p-3 text-left transition-colors {selectedOption ===
        'password'
          ? 'border-primary bg-accent'
          : ''}"
        onclick={() => (selectedOption = "password")}
        type="button"
      >
        <div class="flex items-center gap-3">
          <KeyRound class="text-muted-foreground h-4 w-4" />
          <div>
            <p class="text-sm font-medium">
              {m.auth_setup_security_option_password()}
            </p>
            <p class="text-muted-foreground text-xs">
              {m.auth_setup_security_option_password_description()}
            </p>
          </div>
        </div>
      </button>

      <button
        class="hover:bg-accent w-full rounded-lg border p-3 text-left transition-colors {selectedOption ===
        'skip'
          ? 'border-primary bg-accent'
          : ''}"
        onclick={() => (selectedOption = "skip")}
        type="button"
      >
        <div class="flex items-center gap-3">
          <SkipForward class="text-muted-foreground h-4 w-4" />
          <div>
            <p class="text-sm font-medium">
              {m.auth_setup_security_option_skip()}
            </p>
            <p class="text-muted-foreground text-xs">
              {m.auth_setup_security_option_skip_description()}
            </p>
          </div>
        </div>
      </button>
    </div>

    {#if error}
      <div transition:slide={{duration: 150}}>
        <p class="text-destructive text-center text-sm">{error}</p>
      </div>
    {/if}

    <Button
      class="h-10 w-full"
      disabled={isLoading || !selectedOption}
      onclick={handleContinue}
    >
      {#if isLoading}
        <Loader2 class="mr-2 h-4 w-4 animate-spin" />
        {m.auth_setup_security_button_loading()}
      {:else}
        {m.common_button_continue()}
      {/if}
    </Button>
  </div>
</div>
