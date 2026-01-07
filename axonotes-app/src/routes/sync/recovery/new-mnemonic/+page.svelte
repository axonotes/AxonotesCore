<script lang="ts">
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {onMount} from "svelte";
  import {get} from "svelte/store";
  import {Button} from "$lib/components/ui/button";
  import MnemonicDisplay from "$lib/components/auth/MnemonicDisplay.svelte";
  import ConfirmPhraseModal from "$lib/components/auth/ConfirmPhraseModal.svelte";
  import {recoveryStore} from "$lib/stores/recovery";
  import {app} from "$lib/stores/app";
  import {Key, AlertTriangle} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  let showConfirmModal = $state(false);
  let words = $state<string[]>([]);

  onMount(() => {
    const state = get(recoveryStore);

    // Guard: Must have new mnemonic from previous step
    if (!state.newMnemonic) {
      goto(resolve("/sync/recovery"), {replaceState: true});
      return;
    }

    words = state.newMnemonic.split(" ");
  });

  function handleSavedClick() {
    showConfirmModal = true;
  }

  async function handleConfirm() {
    showConfirmModal = false;

    // Clear recovery state
    recoveryStore.reset();

    // Start local security setup flow
    app.startLocalSecuritySetup();

    goto(resolve("/setup/local-security"));
  }

  function handleCancel() {
    showConfirmModal = false;
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
        <Key class="text-muted-foreground h-5 w-5" />
      </div>
      <div class="space-y-2">
        <h1 class="text-xl font-semibold tracking-tight">
          {m.auth_recovery_newmnemonic_title()}
        </h1>
      </div>
    </div>

    <!-- Warning -->
    <div
      class="border-destructive/30 bg-destructive/5 flex items-start gap-3 rounded-lg border p-4"
    >
      <AlertTriangle class="text-destructive mt-0.5 h-5 w-5 shrink-0" />
      <div class="space-y-1">
        <p class="text-destructive text-sm font-medium">
          {m.common_alert_important()}
        </p>
        <p class="text-muted-foreground text-sm">
          {m.auth_recovery_newmnemonic_warning()}
        </p>
      </div>
    </div>

    <!-- Mnemonic Display -->
    <div class="space-y-5">
      {#if words.length > 0}
        <MnemonicDisplay {words} />
      {/if}

      <Button class="h-10 w-full" onclick={handleSavedClick}>
        {m.auth_setup_mnemonic_button_saved()}
      </Button>
    </div>
  </div>
</div>

<ConfirmPhraseModal
  bind:open={showConfirmModal}
  oncancel={handleCancel}
  onconfirm={handleConfirm}
/>
