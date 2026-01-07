<script lang="ts">
  import {slide} from "svelte/transition";
  import {DatabaseService} from "$lib/services/database";
  import {EncryptionService} from "$lib/services/encryption";
  import {databaseMode} from "$lib/stores/app";
  import {Button} from "$lib/components/ui/button";
  import {Label} from "$lib/components/ui/label";
  import {Textarea} from "$lib/components/ui/textarea";
  import * as Dialog from "$lib/components/ui/dialog";
  import PinInput from "$lib/components/auth/PinInput.svelte";
  import PasswordInput from "$lib/components/auth/PasswordInput.svelte";
  import MnemonicDisplay from "$lib/components/auth/MnemonicDisplay.svelte";
  import {
    ShieldCheck,
    ShieldAlert,
    ChevronRight,
    Loader2,
    Trash2,
    Key,
    KeyRound,
    Lock,
  } from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  // Modal states
  let localEncryptionOpen = $state(false);
  let removeEncryptionOpen = $state(false);
  let masterPasswordOpen = $state(false);
  let mnemonicRegenOpen = $state(false);
  let dangerOpen = $state(false);

  // Local encryption state
  let localPassword = $state("");
  let localConfirmPassword = $state("");
  let localMode = $state<"pin" | "pass">("pass");
  let localError = $state("");
  let localSuccess = $state("");
  let localLoading = $state(false);

  // Master password change state
  let masterMethod = $state<"password" | "mnemonic">("password");
  let masterCurrentPassword = $state("");
  let masterMnemonic = $state("");
  let masterNewPassword = $state("");
  let masterConfirmPassword = $state("");
  let masterError = $state("");
  let masterLoading = $state(false);
  let newMnemonicWords = $state<string[]>([]);

  // Mnemonic regeneration state
  let regenPassword = $state("");
  let regenError = $state("");
  let regenLoading = $state(false);
  let regenMnemonicWords = $state<string[]>([]);

  // Remove encryption state
  let removeLoading = $state(false);
  let removeError = $state("");

  // Danger state
  let dangerLoading = $state(false);

  // Derived
  const isEncrypted = $derived($databaseMode !== "none");
  const modeLabel = $derived(
    $databaseMode === "pin"
      ? "PIN"
      : $databaseMode === "pass"
        ? "Password"
        : "None"
  );

  // Mnemonic validation
  let mnemonicWords = $derived(
    masterMnemonic.trim().toLowerCase().split(/\s+/).filter(Boolean)
  );
  let mnemonicValid = $derived(mnemonicWords.length === 12);

  // Reset modal state when closed
  function resetLocalModal() {
    localPassword = "";
    localConfirmPassword = "";
    localMode = "pass";
    localError = "";
    localSuccess = "";
  }

  function resetMasterModal() {
    masterMethod = "password";
    masterCurrentPassword = "";
    masterMnemonic = "";
    masterNewPassword = "";
    masterConfirmPassword = "";
    masterError = "";
    newMnemonicWords = [];
  }

  function resetRegenModal() {
    regenPassword = "";
    regenError = "";
    regenMnemonicWords = [];
  }

  // Local encryption handlers
  async function handleSetLocalEncryption() {
    localError = "";
    localSuccess = "";
    localLoading = true;

    try {
      if (!localPassword) {
        localError = m.settings_encryption_error_empty();
        return;
      }

      if (localPassword !== localConfirmPassword) {
        localError = m.settings_encryption_error_mismatch();
        return;
      }

      if (
        localMode === "pin" &&
        (localPassword.length !== 6 || !/^[0-9A-Z]+$/.test(localPassword))
      ) {
        localError = m.settings_encryption_error_pin_format();
        return;
      }

      await DatabaseService.setEncryption(localPassword, localMode);
      localSuccess = isEncrypted
        ? m.settings_encryption_success_change({mode: localMode})
        : m.settings_encryption_success_set({mode: localMode});

      setTimeout(() => window.location.reload(), 1500);
    } catch (err: unknown) {
      localError = err instanceof Error ? err.message : String(err);
    } finally {
      localLoading = false;
    }
  }

  async function handleRemoveLocalEncryption() {
    removeError = "";
    removeLoading = true;

    try {
      await DatabaseService.removeEncryption();
      setTimeout(() => window.location.reload(), 1500);
    } catch (err: unknown) {
      removeError = err instanceof Error ? err.message : String(err);
    } finally {
      removeLoading = false;
    }
  }

  // Master password change handler
  async function handleChangeMasterPassword() {
    masterError = "";
    newMnemonicWords = [];
    masterLoading = true;

    try {
      if (!masterNewPassword) {
        masterError = m.settings_encryption_error_empty();
        return;
      }

      if (masterNewPassword !== masterConfirmPassword) {
        masterError = m.settings_encryption_error_mismatch();
        return;
      }

      if (masterMethod === "password") {
        if (!masterCurrentPassword) {
          masterError = m.settings_encryption_error_empty();
          return;
        }

        await EncryptionService.updatePasswordFromPassword(
          masterCurrentPassword,
          masterNewPassword
        );

        // Success - close modal
        masterPasswordOpen = false;
        resetMasterModal();
      } else {
        if (!mnemonicValid) {
          masterError = m.auth_recovery_error_invalid();
          return;
        }

        const cleanMnemonic = mnemonicWords.join(" ");
        const newMnemonic = await EncryptionService.updatePasswordFromMnemonic(
          cleanMnemonic,
          masterNewPassword
        );

        newMnemonicWords = newMnemonic.split(" ");
        masterMnemonic = "";
        masterNewPassword = "";
        masterConfirmPassword = "";
      }
    } catch (err: unknown) {
      masterError = err instanceof Error ? err.message : String(err);
    } finally {
      masterLoading = false;
    }
  }

  // Mnemonic regeneration handler
  async function handleRegenerateMnemonic() {
    regenError = "";
    regenMnemonicWords = [];
    regenLoading = true;

    try {
      if (!regenPassword) {
        regenError = m.settings_encryption_error_empty();
        return;
      }

      const newMnemonic =
        await EncryptionService.updateMnemonicFromPassword(regenPassword);

      regenMnemonicWords = newMnemonic.split(" ");
      regenPassword = "";
    } catch (err: unknown) {
      regenError = err instanceof Error ? err.message : String(err);
    } finally {
      regenLoading = false;
    }
  }

  // Danger zone handler
  async function handleWipe() {
    dangerLoading = true;

    try {
      await DatabaseService.wipe();
      window.location.href = "/unlock";
    } catch {
      dangerLoading = false;
    }
  }
</script>

<div class="flex h-full w-full justify-center overflow-auto p-8">
  <div class="w-full max-w-md space-y-8">
    <!-- Header -->
    <div class="space-y-2">
      <h1 class="text-2xl leading-tight font-semibold tracking-tight">
        {m.settings_security_title()}
      </h1>
      <p class="text-muted-foreground text-sm leading-normal">
        {m.settings_security_description()}
      </p>
    </div>

    <!-- Local Database Status -->
    <section class="space-y-4">
      <button
        onclick={() => {
          resetLocalModal();
          localEncryptionOpen = true;
        }}
        class="hover:bg-accent/50 w-full rounded-lg border p-4 text-left transition-colors"
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            {#if isEncrypted}
              <ShieldCheck class="h-5 w-5 text-green-600 dark:text-green-400" />
            {:else}
              <ShieldAlert
                class="h-5 w-5 text-yellow-600 dark:text-yellow-400"
              />
            {/if}
            <div>
              <p class="text-sm font-medium">
                {m.settings_encryption_title_set()}
              </p>
              <p class="text-muted-foreground text-xs">
                {m.settings_security_mode_label()}: {modeLabel}
              </p>
            </div>
          </div>
          <ChevronRight class="text-muted-foreground h-4 w-4" />
        </div>
      </button>

      {#if !isEncrypted}
        <div
          class="flex items-start gap-3 rounded-lg border border-yellow-500/30 bg-yellow-500/10 p-3"
        >
          <ShieldAlert
            class="mt-0.5 h-4 w-4 shrink-0 text-yellow-600 dark:text-yellow-400"
          />
          <p
            class="text-xs leading-normal text-yellow-700 dark:text-yellow-300"
          >
            {m.settings_security_unencrypted_warning()}
          </p>
        </div>
      {/if}
    </section>

    <!-- Master Password -->
    <section>
      <button
        onclick={() => {
          resetMasterModal();
          masterPasswordOpen = true;
        }}
        class="hover:bg-accent/50 w-full rounded-lg border p-4 text-left transition-colors"
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <Key class="text-muted-foreground h-5 w-5" />
            <div>
              <p class="text-sm font-medium">
                {m.settings_master_change_title()}
              </p>
              <p class="text-muted-foreground text-xs">
                {m.settings_master_change_description()}
              </p>
            </div>
          </div>
          <ChevronRight class="text-muted-foreground h-4 w-4" />
        </div>
      </button>
    </section>

    <!-- Mnemonic Regeneration -->
    <section>
      <button
        onclick={() => {
          resetRegenModal();
          mnemonicRegenOpen = true;
        }}
        class="hover:bg-accent/50 w-full rounded-lg border p-4 text-left transition-colors"
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <KeyRound class="text-muted-foreground h-5 w-5" />
            <div>
              <p class="text-sm font-medium">{m.settings_mnemonic_title()}</p>
              <p class="text-muted-foreground text-xs">
                {m.settings_mnemonic_description()}
              </p>
            </div>
          </div>
          <ChevronRight class="text-muted-foreground h-4 w-4" />
        </div>
      </button>
    </section>

    <!-- Danger Zone -->
    <section>
      <button
        onclick={() => (dangerOpen = true)}
        class="border-destructive/30 hover:bg-destructive/5 w-full rounded-lg border p-4 text-left transition-colors"
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <Trash2 class="text-destructive h-5 w-5" />
            <div>
              <p class="text-destructive text-sm font-medium">
                {m.settings_danger_title()}
              </p>
              <p class="text-muted-foreground text-xs">
                {m.settings_danger_description()}
              </p>
            </div>
          </div>
          <ChevronRight class="text-muted-foreground h-4 w-4" />
        </div>
      </button>
    </section>
  </div>
</div>

<!-- Local Encryption Modal -->
<Dialog.Root bind:open={localEncryptionOpen}>
  <Dialog.Content class="max-w-sm">
    <Dialog.Header>
      <Dialog.Title>
        {isEncrypted
          ? m.settings_encryption_title_change()
          : m.settings_encryption_title_set()}
      </Dialog.Title>
      <Dialog.Description>
        {isEncrypted
          ? m.settings_encryption_description_change()
          : m.settings_encryption_description_set()}
      </Dialog.Description>
    </Dialog.Header>

    <div class="space-y-5 py-4">
      <!-- Mode Selection -->
      <div class="space-y-3">
        <button
          class="hover:bg-accent w-full rounded-lg border p-3 text-left transition-colors {localMode ===
          'pin'
            ? 'border-primary bg-accent'
            : ''}"
          onclick={() => (localMode = "pin")}
          type="button"
        >
          <div class="flex items-center gap-3">
            <Lock class="text-muted-foreground h-4 w-4" />
            <div>
              <p class="text-sm font-medium">
                {m.settings_encryption_mode_pin()}
              </p>
              <p class="text-muted-foreground text-xs">
                {m.auth_setup_security_option_pin_description()}
              </p>
            </div>
          </div>
        </button>

        <button
          class="hover:bg-accent w-full rounded-lg border p-3 text-left transition-colors {localMode ===
          'pass'
            ? 'border-primary bg-accent'
            : ''}"
          onclick={() => (localMode = "pass")}
          type="button"
        >
          <div class="flex items-center gap-3">
            <KeyRound class="text-muted-foreground h-4 w-4" />
            <div>
              <p class="text-sm font-medium">
                {m.settings_encryption_mode_password()}
              </p>
              <p class="text-muted-foreground text-xs">
                {m.auth_setup_security_option_password_description()}
              </p>
            </div>
          </div>
        </button>
      </div>

      <!-- PIN or Password Input -->
      {#if localMode === "pin"}
        <PinInput
          id="local-pin"
          label={m.settings_encryption_new_label_pin()}
          bind:value={localPassword}
          disabled={localLoading}
        />
        <PinInput
          id="local-pin-confirm"
          label={m.settings_encryption_confirm_label_pin()}
          bind:value={localConfirmPassword}
          disabled={localLoading}
        />
      {:else}
        <PasswordInput
          id="local-password"
          label={m.settings_encryption_new_label_password()}
          placeholder={m.settings_encryption_new_placeholder_password()}
          bind:value={localPassword}
          disabled={localLoading}
          showStrength
        />
        <PasswordInput
          id="local-password-confirm"
          label={m.settings_encryption_confirm_label_password()}
          placeholder={m.auth_setup_master_confirm_placeholder()}
          bind:value={localConfirmPassword}
          disabled={localLoading}
        />
      {/if}

      {#if localError}
        <p class="text-destructive text-sm">{localError}</p>
      {/if}
      {#if localSuccess}
        <p class="text-sm text-green-600 dark:text-green-400">{localSuccess}</p>
      {/if}
    </div>

    <Dialog.Footer class="flex-col gap-2 sm:flex-col">
      <Button
        class="w-full"
        onclick={handleSetLocalEncryption}
        disabled={localLoading}
      >
        {#if localLoading}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
        {/if}
        {isEncrypted
          ? m.settings_encryption_button_change()
          : m.settings_encryption_button_set()}
      </Button>
      {#if isEncrypted}
        <Button
          variant="outline"
          class="w-full"
          onclick={() => {
            localEncryptionOpen = false;
            removeEncryptionOpen = true;
          }}
          disabled={localLoading}
        >
          {m.settings_encryption_button_remove()}
        </Button>
      {/if}
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<!-- Remove Encryption Modal -->
<Dialog.Root bind:open={removeEncryptionOpen}>
  <Dialog.Content class="max-w-xs">
    <Dialog.Header>
      <Dialog.Title>{m.settings_encryption_button_remove()}</Dialog.Title>
      <Dialog.Description
        >{m.settings_danger_remove_confirm()}</Dialog.Description
      >
    </Dialog.Header>

    {#if removeError}
      <p class="text-destructive py-2 text-sm">{removeError}</p>
    {/if}

    <Dialog.Footer class="flex-col gap-2 sm:flex-col">
      <Button
        variant="outline"
        class="w-full"
        onclick={() => (removeEncryptionOpen = false)}
      >
        {m.common_button_cancel()}
      </Button>
      <Button
        variant="destructive"
        class="w-full"
        onclick={handleRemoveLocalEncryption}
        disabled={removeLoading}
      >
        {#if removeLoading}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
        {/if}
        {m.settings_encryption_button_remove()}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<!-- Master Password Modal -->
<Dialog.Root bind:open={masterPasswordOpen}>
  <Dialog.Content class="max-w-sm">
    <Dialog.Header>
      <Dialog.Title>{m.settings_master_change_title()}</Dialog.Title>
      <Dialog.Description
        >{m.settings_master_change_description()}</Dialog.Description
      >
    </Dialog.Header>

    <div class="space-y-5 py-4">
      {#if newMnemonicWords.length === 0}
        <!-- Method Selection -->
        <div class="flex gap-2">
          <button
            class="hover:bg-accent flex-1 rounded-lg border px-3 py-2 text-sm transition-colors {masterMethod ===
            'password'
              ? 'border-primary bg-accent'
              : ''}"
            onclick={() => (masterMethod = "password")}
            type="button"
          >
            {m.settings_master_method_password()}
          </button>
          <button
            class="hover:bg-accent flex-1 rounded-lg border px-3 py-2 text-sm transition-colors {masterMethod ===
            'mnemonic'
              ? 'border-primary bg-accent'
              : ''}"
            onclick={() => (masterMethod = "mnemonic")}
            type="button"
          >
            {m.settings_master_method_mnemonic()}
          </button>
        </div>

        <!-- Current Password or Mnemonic -->
        {#if masterMethod === "password"}
          <PasswordInput
            id="master-current-password"
            label={m.settings_master_current_password_label()}
            bind:value={masterCurrentPassword}
            disabled={masterLoading}
          />
        {:else}
          <div class="space-y-2">
            <Label for="master-mnemonic"
              >{m.settings_master_mnemonic_label()}</Label
            >
            <Textarea
              bind:value={masterMnemonic}
              class="min-h-[80px] resize-none font-mono text-sm"
              disabled={masterLoading}
              id="master-mnemonic"
              placeholder={m.settings_master_mnemonic_placeholder()}
            />
            <p class="text-muted-foreground text-xs">
              {mnemonicWords.length}/12 words
            </p>
          </div>
        {/if}

        <!-- New Password -->
        <PasswordInput
          id="master-new-password"
          label={m.settings_master_new_password_label()}
          bind:value={masterNewPassword}
          disabled={masterLoading}
          showStrength
        />

        <PasswordInput
          id="master-confirm-password"
          label={m.settings_master_confirm_password_label()}
          bind:value={masterConfirmPassword}
          disabled={masterLoading}
        />

        {#if masterError}
          <p class="text-destructive text-sm">{masterError}</p>
        {/if}
      {:else}
        <div class="space-y-4" transition:slide={{duration: 150}}>
          <div
            class="border-destructive/30 bg-destructive/10 flex items-start gap-3 rounded-lg border p-3"
          >
            <ShieldAlert class="text-destructive mt-0.5 h-4 w-4 shrink-0" />
            <p class="text-destructive text-xs leading-normal">
              {m.auth_recovery_newmnemonic_warning()}
            </p>
          </div>
          <MnemonicDisplay words={newMnemonicWords} />
        </div>
      {/if}
    </div>

    <Dialog.Footer>
      {#if newMnemonicWords.length === 0}
        <Button
          class="w-full"
          onclick={handleChangeMasterPassword}
          disabled={masterLoading ||
            (masterMethod === "mnemonic" && !mnemonicValid)}
        >
          {#if masterLoading}
            <Loader2 class="mr-2 h-4 w-4 animate-spin" />
          {/if}
          {m.settings_master_button_change()}
        </Button>
      {:else}
        <Button class="w-full" onclick={() => (masterPasswordOpen = false)}>
          {m.auth_setup_mnemonic_button_saved()}
        </Button>
      {/if}
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<!-- Mnemonic Regeneration Modal -->
<Dialog.Root bind:open={mnemonicRegenOpen}>
  <Dialog.Content class="max-w-sm">
    <Dialog.Header>
      <Dialog.Title>{m.settings_mnemonic_title()}</Dialog.Title>
      <Dialog.Description
        >{m.settings_mnemonic_description()}</Dialog.Description
      >
    </Dialog.Header>

    <div class="space-y-5 py-4">
      {#if regenMnemonicWords.length === 0}
        <div
          class="flex items-start gap-3 rounded-lg border border-yellow-500/30 bg-yellow-500/10 p-3"
        >
          <ShieldAlert
            class="mt-0.5 h-4 w-4 shrink-0 text-yellow-600 dark:text-yellow-400"
          />
          <p
            class="text-xs leading-normal text-yellow-700 dark:text-yellow-300"
          >
            {m.settings_mnemonic_warning()}
          </p>
        </div>

        <PasswordInput
          id="regen-password"
          label={m.settings_mnemonic_password_label()}
          bind:value={regenPassword}
          disabled={regenLoading}
        />

        {#if regenError}
          <p class="text-destructive text-sm">{regenError}</p>
        {/if}
      {:else}
        <div class="space-y-4" transition:slide={{duration: 150}}>
          <p class="text-sm text-green-600 dark:text-green-400">
            {m.settings_mnemonic_success()}
          </p>
          <MnemonicDisplay words={regenMnemonicWords} />
        </div>
      {/if}
    </div>

    <Dialog.Footer>
      {#if regenMnemonicWords.length === 0}
        <Button
          class="w-full"
          onclick={handleRegenerateMnemonic}
          disabled={regenLoading || !regenPassword}
        >
          {#if regenLoading}
            <Loader2 class="mr-2 h-4 w-4 animate-spin" />
          {/if}
          {m.settings_mnemonic_button_regenerate()}
        </Button>
      {:else}
        <Button class="w-full" onclick={() => (mnemonicRegenOpen = false)}>
          {m.auth_setup_mnemonic_button_saved()}
        </Button>
      {/if}
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<!-- Danger Zone Modal -->
<Dialog.Root bind:open={dangerOpen}>
  <Dialog.Content class="max-w-xs">
    <Dialog.Header>
      <Dialog.Title class="text-destructive"
        >{m.settings_danger_wipe_button()}</Dialog.Title
      >
      <Dialog.Description class="space-y-2">
        <span class="block">{m.settings_danger_wipe_description()}</span>
        <span class="block text-xs">{m.settings_danger_wipe_safe()}</span>
      </Dialog.Description>
    </Dialog.Header>

    <Dialog.Footer class="flex-col gap-2 sm:flex-col">
      <Button
        variant="outline"
        class="w-full"
        onclick={() => (dangerOpen = false)}
      >
        {m.common_button_cancel()}
      </Button>
      <Button
        variant="destructive"
        class="w-full"
        onclick={handleWipe}
        disabled={dangerLoading}
      >
        {#if dangerLoading}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
        {/if}
        {m.settings_danger_wipe_button()}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
