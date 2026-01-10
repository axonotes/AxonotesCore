<script lang="ts">
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {onMount} from "svelte";
  import {get} from "svelte/store";
  import {slide} from "svelte/transition";
  import {Button} from "$lib/components/ui/button";
  import PinInput from "$lib/components/auth/PinInput.svelte";
  import {setupStore} from "$lib/stores/setup";
  import {DatabaseService} from "$lib/services/database";
  import {app, isReady, isSettingUpLocalSecurity} from "$lib/stores/app";
  import {Hash, Loader2, ArrowLeft} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  let pinValue = $state("");
  let pinConfirm = $state("");
  let error = $state("");
  let isLoading = $state(false);
  let isFromSyncFlow = $state(false);
  let pinInput: {focus: () => void; focusLast: () => void};
  let confirmInput: {focus: () => void};

  let pinValid = $derived(pinValue.length === 6 && pinValue === pinConfirm);

  onMount(() => {
    const state = get(setupStore);
    const settingUpLocal = get(isSettingUpLocalSecurity);
    const ready = get(isReady);

    // Case 1: Coming from sync/recovery flow
    if (settingUpLocal) {
      isFromSyncFlow = true;
      return;
    }

    // Case 2: Coming from new user setup
    if (state.mnemonic) {
      isFromSyncFlow = false;
      return;
    }

    // Case 3: Already set up - redirect to app
    if (ready) {
      goto(resolve("/app"), {replaceState: true});
      return;
    }

    // Case 4: Invalid access - redirect to local-security selection
    goto(resolve("/setup/local-security"), {replaceState: true});
  });

  async function handleFinish() {
    error = "";

    if (pinValue.length !== 6) {
      error = m.auth_setup_pin_error_length();
      return;
    }

    if (!pinValid) {
      error = m.auth_setup_pin_error_mismatch();
      return;
    }

    isLoading = true;

    try {
      await DatabaseService.setEncryption(pinValue, "pin");
      setupStore.reset();
      await app.finishLocalSecuritySetup();
      goto(resolve("/app"), {replaceState: true});
    } catch (err) {
      console.error("Failed to set encryption:", err);
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
        <Hash class="text-muted-foreground h-5 w-5" />
      </div>
      <div class="space-y-2">
        {#if !isFromSyncFlow}
          <p
            class="text-muted-foreground text-xs font-medium tracking-wide uppercase"
          >
            {m.auth_setup_step({current: 3, total: 3})}
          </p>
        {/if}
        <h1 class="text-3xl font-semibold tracking-tight">
          {m.auth_setup_pin_title()}
        </h1>
        <p class="text-muted-foreground text-sm">
          {m.auth_setup_pin_description()}
        </p>
      </div>
    </div>

    <!-- Form -->
    <form
      class="space-y-5"
      onsubmit={(e) => {
        e.preventDefault();
        handleFinish();
      }}
    >
      <PinInput
        autofocus
        bind:this={pinInput}
        bind:value={pinValue}
        disabled={isLoading}
        id="pin"
        label={m.auth_setup_pin_label()}
        oncomplete={() => confirmInput?.focus()}
      />

      <PinInput
        bind:this={confirmInput}
        bind:value={pinConfirm}
        disabled={isLoading}
        id="pin-confirm"
        label={m.auth_setup_pin_confirm_label()}
        onbackspaceatstart={() => pinInput?.focusLast()}
        onenter={handleFinish}
      />

      {#if error}
        <div transition:slide={{duration: 150}}>
          <p class="text-destructive text-center text-sm">{error}</p>
        </div>
      {/if}

      <div class="flex gap-2">
        <Button
          class="h-10"
          disabled={isLoading}
          onclick={() => goto(resolve("/setup/local-security"))}
          type="button"
          variant="outline"
        >
          <ArrowLeft class="mr-2 h-4 w-4" />
          {m.common_button_back()}
        </Button>
        <Button
          class="h-10 flex-1"
          disabled={isLoading || !pinValid}
          type="submit"
        >
          {#if isLoading}
            <Loader2 class="mr-2 h-4 w-4 animate-spin" />
            {m.auth_setup_security_button_loading()}
          {:else}
            {m.common_button_continue()}
          {/if}
        </Button>
      </div>
    </form>
  </div>
</div>
