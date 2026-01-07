<script lang="ts">
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {slide} from "svelte/transition";
  import {Button} from "$lib/components/ui/button";
  import {Textarea} from "$lib/components/ui/textarea";
  import {Label} from "$lib/components/ui/label";
  import {EncryptionService} from "$lib/services/encryption";
  import {recoveryStore} from "$lib/stores/recovery";
  import {KeyRound, Loader2, ArrowLeft} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  let mnemonic = $state("");
  let error = $state("");
  let isLoading = $state(false);

  // Validate mnemonic format (12 words)
  let mnemonicWords = $derived(
    mnemonic.trim().toLowerCase().split(/\s+/).filter(Boolean)
  );
  let mnemonicValid = $derived(mnemonicWords.length === 12);

  async function handleRecover() {
    error = "";

    if (!mnemonicValid) {
      error = m.auth_recovery_error_invalid();
      return;
    }

    isLoading = true;

    try {
      // Sync keys using mnemonic (validates the mnemonic)
      const cleanMnemonic = mnemonicWords.join(" ");
      await EncryptionService.syncWithMnemonic(cleanMnemonic);

      // Store mnemonic for password reset step
      recoveryStore.setOldMnemonic(cleanMnemonic);
      recoveryStore.setStep(2);

      goto(resolve("/sync/recovery/new-password"));
    } catch (err) {
      console.error("Failed to recover with mnemonic:", err);
      error = m.auth_recovery_error_invalid();
    } finally {
      isLoading = false;
    }
  }
</script>

<div
  class="bg-background flex min-h-full w-full items-center justify-center p-6"
>
  <div class="w-full max-w-md space-y-8">
    <!-- Header -->
    <div class="space-y-4 text-center">
      <div
        class="bg-muted mx-auto flex h-12 w-12 items-center justify-center rounded-full"
      >
        <KeyRound class="text-muted-foreground h-5 w-5" />
      </div>
      <div class="space-y-2">
        <h1 class="text-xl font-semibold tracking-tight">
          {m.auth_recovery_title()}
        </h1>
        <p class="text-muted-foreground text-sm">
          {m.auth_recovery_description()}
        </p>
      </div>
    </div>

    <!-- Form -->
    <form
      class="space-y-5"
      onsubmit={(e) => {
        e.preventDefault();
        handleRecover();
      }}
    >
      <div class="space-y-2">
        <Label for="mnemonic">{m.auth_recovery_mnemonic_label()}</Label>
        <Textarea
          bind:value={mnemonic}
          class="min-h-[100px] resize-none font-mono text-sm"
          disabled={isLoading}
          id="mnemonic"
          placeholder={m.auth_recovery_mnemonic_placeholder()}
        />
        <p class="text-muted-foreground text-xs">
          {mnemonicWords.length}/12 words
        </p>
      </div>

      {#if error}
        <div transition:slide={{duration: 150}}>
          <p class="text-destructive text-center text-sm">{error}</p>
        </div>
      {/if}

      <div class="flex gap-2">
        <Button
          class="h-10"
          disabled={isLoading}
          onclick={() => goto(resolve("/sync"))}
          type="button"
          variant="outline"
        >
          <ArrowLeft class="mr-2 h-4 w-4" />
          {m.common_button_back()}
        </Button>
        <Button
          class="h-10 flex-1"
          disabled={isLoading || !mnemonicValid}
          type="submit"
        >
          {#if isLoading}
            <Loader2 class="mr-2 h-4 w-4 animate-spin" />
            {m.auth_recovery_button_loading()}
          {:else}
            {m.auth_recovery_button_primary()}
          {/if}
        </Button>
      </div>
    </form>
  </div>
</div>
