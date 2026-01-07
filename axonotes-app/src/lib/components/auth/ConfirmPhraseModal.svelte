<script lang="ts">
  import {Dialog, DialogContent} from "$lib/components/ui/dialog";
  import {Button} from "$lib/components/ui/button";
  import {Input} from "$lib/components/ui/input";
  import {Label} from "$lib/components/ui/label";
  import {AlertTriangle} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  interface Props {
    open: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  }

  let {open = $bindable(false), onconfirm, oncancel}: Props = $props();

  const REQUIRED_PHRASE = "I lose my data if I lose both";
  let inputValue = $state("");
  let hasAttempted = $state(false);

  let isMatch = $derived(
    inputValue.toLowerCase().trim() === REQUIRED_PHRASE.toLowerCase()
  );

  function handleConfirm() {
    hasAttempted = true;
    if (isMatch) {
      inputValue = "";
      hasAttempted = false;
      onconfirm();
    }
  }

  function handleCancel() {
    inputValue = "";
    hasAttempted = false;
    oncancel();
  }

  function handleOpenChange(newOpen: boolean) {
    if (!newOpen) {
      handleCancel();
    }
    open = newOpen;
  }
</script>

<Dialog onOpenChange={handleOpenChange} {open}>
  <DialogContent class="max-w-sm gap-0 p-6">
    <div class="space-y-6">
      <!-- Header -->
      <div class="space-y-4 text-center">
        <div
          class="bg-muted mx-auto flex h-12 w-12 items-center justify-center rounded-full"
        >
          <AlertTriangle class="text-muted-foreground h-5 w-5" />
        </div>
        <div class="space-y-2">
          <h2 class="text-xl font-semibold tracking-tight">
            {m.auth_setup_confirm_title()}
          </h2>
          <p class="text-muted-foreground text-sm">
            {m.auth_setup_confirm_description()}
          </p>
        </div>
      </div>

      <!-- Phrase to type -->
      <div class="bg-muted rounded-lg border p-4">
        <p class="text-center font-mono text-sm font-medium">
          {m.auth_setup_confirm_phrase()}
        </p>
      </div>

      <!-- Input field -->
      <div class="space-y-2">
        <Label for="confirm-phrase">{m.common_label_type_phrase()}</Label>
        <Input
          bind:value={inputValue}
          id="confirm-phrase"
          onkeydown={(e) => {
            if (e.key === "Enter") {
              handleConfirm();
            }
          }}
          placeholder={m.auth_setup_confirm_input_placeholder()}
        />
        {#if hasAttempted && !isMatch}
          <p class="text-destructive text-sm">
            {m.auth_setup_confirm_error_mismatch()}
          </p>
        {/if}
      </div>

      <!-- Buttons -->
      <div class="flex gap-2">
        <Button class="h-10 flex-1" onclick={handleCancel} variant="outline">
          {m.common_button_cancel()}
        </Button>
        <Button
          class="h-10 flex-1"
          disabled={!inputValue.trim()}
          onclick={handleConfirm}
        >
          {m.common_button_confirm()}
        </Button>
      </div>
    </div>
  </DialogContent>
</Dialog>
