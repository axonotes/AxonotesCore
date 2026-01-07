<script lang="ts">
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {onMount} from "svelte";
  import {get} from "svelte/store";
  import {slide} from "svelte/transition";
  import {Button} from "$lib/components/ui/button";
  import PasswordInput from "$lib/components/auth/PasswordInput.svelte";
  import {EncryptionService} from "$lib/services/encryption";
  import {recoveryStore} from "$lib/stores/recovery";
  import {Shield, Loader2, ArrowLeft} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";
  import zxcvbn from "zxcvbn";

  let password = $state("");
  let confirmPassword = $state("");
  let error = $state("");
  let isLoading = $state(false);

  // Password validation
  let passwordsMatch = $derived(password === confirmPassword);
  let strength = $derived(password ? zxcvbn(password) : null);
  let isValid = $derived(password.length > 0 && passwordsMatch);

  onMount(() => {
    const state = get(recoveryStore);

    // Guard: Must have mnemonic from previous step
    if (!state.oldMnemonic) {
      goto(resolve("/sync/recovery"), {replaceState: true});
      return;
    }
  });

  async function handleSetPassword() {
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
      const state = get(recoveryStore);

      // Update password and get new mnemonic
      const newMnemonic = await EncryptionService.updatePasswordFromMnemonic(
        state.oldMnemonic,
        password
      );

      // Store new mnemonic for display
      recoveryStore.setNewPassword(password);
      recoveryStore.setNewMnemonic(newMnemonic);
      recoveryStore.setStep(3);

      goto(resolve("/sync/recovery/new-mnemonic"));
    } catch (err) {
      console.error("Failed to set new password:", err);
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
        <h1 class="text-xl font-semibold tracking-tight">
          {m.auth_recovery_newpwd_title()}
        </h1>
        <p class="text-muted-foreground text-sm">
          {m.auth_recovery_newpwd_description()}
        </p>
      </div>
    </div>

    <!-- Form -->
    <form
      class="space-y-5"
      onsubmit={(e) => {
        e.preventDefault();
        handleSetPassword();
      }}
    >
      <PasswordInput
        autofocus
        bind:value={password}
        disabled={isLoading}
        id="new-password"
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

      {#if error}
        <div transition:slide={{duration: 150}}>
          <p class="text-destructive text-center text-sm">{error}</p>
        </div>
      {/if}

      <div class="flex gap-2">
        <Button
          class="h-10"
          disabled={isLoading}
          onclick={() => goto(resolve("/sync/recovery"))}
          type="button"
          variant="outline"
        >
          <ArrowLeft class="mr-2 h-4 w-4" />
          {m.common_button_back()}
        </Button>
        <Button
          class="h-10 flex-1"
          disabled={isLoading || !isValid}
          type="submit"
        >
          {#if isLoading}
            <Loader2 class="mr-2 h-4 w-4 animate-spin" />
            {m.auth_recovery_newpwd_button_loading()}
          {:else}
            {m.common_button_next()}
          {/if}
        </Button>
      </div>
    </form>
  </div>
</div>
