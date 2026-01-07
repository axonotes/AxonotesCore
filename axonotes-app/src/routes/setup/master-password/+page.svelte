<script lang="ts">
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {onMount} from "svelte";
  import {get} from "svelte/store";
  import {slide} from "svelte/transition";
  import {Button} from "$lib/components/ui/button";
  import PasswordInput from "$lib/components/auth/PasswordInput.svelte";
  import {setupStore} from "$lib/stores/setup";
  import {EncryptionService} from "$lib/services/encryption";
  import {needsSetup, isReady} from "$lib/stores/app";
  import {Shield, Loader2, AlertTriangle} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";
  import zxcvbn from "zxcvbn";

  let password = $state("");
  let confirmPassword = $state("");
  let error = $state("");
  let isLoading = $state(false);

  onMount(() => {
    // Guard: Only allow access if setup is actually needed
    // If user is already ready or doesn't need setup, redirect away
    if (!get(needsSetup)) {
      if (get(isReady)) {
        goto(resolve("/app"), {replaceState: true});
      } else {
        goto(resolve("/"), {replaceState: true});
      }
      return;
    }

    // Reset setup store when starting fresh
    setupStore.reset();
  });

  // Password validation
  let passwordsMatch = $derived(password === confirmPassword);
  let strength = $derived(password ? zxcvbn(password) : null);
  let isWeak = $derived(strength !== null && strength.score < 2);

  async function handleNext() {
    error = "";

    if (!password) {
      error = m.auth_setup_master_error_empty();
      return;
    }

    if (!passwordsMatch) {
      error = m.auth_setup_master_error_mismatch();
      return;
    }

    isLoading = true;

    try {
      const mnemonic = await EncryptionService.createUser(password);

      setupStore.setMasterPassword(password);
      setupStore.setMnemonic(mnemonic);
      setupStore.setStep(2);

      goto(resolve("/setup/backup-phrase"));
    } catch (err) {
      console.error("[Setup] Failed to create user:", err);
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
        <Shield class="text-muted-foreground h-5 w-5" />
      </div>
      <div class="space-y-2">
        <p
          class="text-muted-foreground text-xs font-medium tracking-wide uppercase"
        >
          {m.auth_setup_step({current: 1, total: 3})}
        </p>
        <h1 class="text-xl font-semibold tracking-tight">
          {m.auth_setup_master_title()}
        </h1>
        <p class="text-muted-foreground text-sm">
          {m.auth_setup_master_description()}
        </p>
      </div>
    </div>

    <!-- Form -->
    <form
      class="space-y-5"
      onsubmit={(e) => {
        e.preventDefault();
        handleNext();
      }}
    >
      <PasswordInput
        autofocus
        bind:value={password}
        disabled={isLoading}
        id="master-password"
        label={m.auth_setup_master_password_label()}
        placeholder={m.auth_setup_master_password_placeholder()}
        showStrength={true}
      />

      <PasswordInput
        bind:value={confirmPassword}
        disabled={isLoading}
        id="confirm-password"
        label={m.auth_setup_master_confirm_label()}
        placeholder={m.auth_setup_master_confirm_placeholder()}
      />

      {#if isWeak && password}
        <div transition:slide={{duration: 150}}>
          <div
            class="flex items-start gap-2 rounded-md border border-yellow-500/20 bg-yellow-500/10 p-3"
          >
            <AlertTriangle
              class="mt-0.5 h-4 w-4 shrink-0 text-yellow-600 dark:text-yellow-400"
            />
            <p class="text-sm text-yellow-600 dark:text-yellow-400">
              {m.auth_setup_master_warning_weak()}
            </p>
          </div>
        </div>
      {/if}

      {#if error}
        <div transition:slide={{duration: 150}}>
          <p class="text-destructive text-center text-sm">{error}</p>
        </div>
      {/if}

      <Button
        class="h-10 w-full"
        disabled={isLoading || !password || !confirmPassword}
        type="submit"
      >
        {#if isLoading}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
          {m.auth_setup_master_button_loading()}
        {:else}
          {m.common_button_next()}
        {/if}
      </Button>
    </form>
  </div>
</div>
