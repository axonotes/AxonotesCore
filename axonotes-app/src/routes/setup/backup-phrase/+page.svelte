<script lang="ts">
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {onMount} from "svelte";
  import {get} from "svelte/store";
  import {Button} from "$lib/components/ui/button";
  import MnemonicDisplay from "$lib/components/auth/MnemonicDisplay.svelte";
  import ConfirmPhraseModal from "$lib/components/auth/ConfirmPhraseModal.svelte";
  import {setupStore} from "$lib/stores/setup";
  import {Key, AlertTriangle} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  let showConfirmModal = $state(false);
  let words = $state<string[]>([]);

  onMount(() => {
    const state = get(setupStore);

    if (!state.mnemonic) {
      goto(resolve("/setup/master-password"), {replaceState: true});
      return;
    }

    words = state.mnemonic.split(" ");
  });

  function handleSavedClick() {
    showConfirmModal = true;
  }

  function handleConfirm() {
    showConfirmModal = false;
    setupStore.setStep(3);
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
        <p
          class="text-muted-foreground text-xs font-medium tracking-wide uppercase"
        >
          {m.auth_setup_step({current: 2, total: 3})}
        </p>
        <h1 class="text-3xl font-semibold tracking-tight">
          {m.auth_setup_mnemonic_title()}
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
          {m.auth_setup_mnemonic_warning_title()}
        </p>
        <p class="text-muted-foreground text-sm">
          {m.auth_setup_mnemonic_warning_text()}
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
