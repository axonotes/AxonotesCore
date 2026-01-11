<script lang="ts">
  import {UserPlus} from "@lucide/svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import {Button} from "$lib/components/ui/button";
  import {Input} from "$lib/components/ui/input";
  import {Label} from "$lib/components/ui/label";
  import {Spinner} from "$lib/components/ui/spinner";
  import {ShareService} from "$lib/services/share";
  import * as m from "$lib/paraglide/messages.js";

  interface Props {
    open: boolean;
    onOpenChange?: (open: boolean) => void;
    onSuccess?: () => void;
  }

  let {open = $bindable(false), onOpenChange, onSuccess}: Props = $props();

  let shareCode = $state("");
  let joining = $state(false);
  let error = $state<string | null>(null);

  // Format share code as user types (uppercase, max 8 chars)
  function handleInput(event: Event) {
    const input = event.target as HTMLInputElement;
    shareCode = input.value
      .toUpperCase()
      .replace(/[^A-Z0-9]/g, "")
      .slice(0, 8);
  }

  async function handleJoin() {
    if (shareCode.length !== 8) {
      error = m.share_error_invalid_code();
      return;
    }

    joining = true;
    error = null;

    try {
      await ShareService.joinShare(shareCode);
      // Success - close dialog
      shareCode = "";
      open = false;
      onOpenChange?.(false);
      onSuccess?.();
    } catch (e) {
      error = e instanceof Error ? e.message : m.share_error_generic();
    } finally {
      joining = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && shareCode.length === 8 && !joining) {
      handleJoin();
    }
  }

  function handleOpenChange(newOpen: boolean) {
    open = newOpen;
    onOpenChange?.(newOpen);
    if (!newOpen) {
      // Reset state when closing
      shareCode = "";
      error = null;
    }
  }
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
  <Dialog.Content class="sm:max-w-md">
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2">
        <UserPlus class="h-5 w-5" />
        {m.share_join_title()}
      </Dialog.Title>
    </Dialog.Header>

    <div class="space-y-4 py-2">
      <div class="space-y-2">
        <Label for="share-code">{m.share_code_label()}</Label>
        <Input
          id="share-code"
          value={shareCode}
          oninput={handleInput}
          onkeydown={handleKeydown}
          placeholder={m.share_join_placeholder()}
          class="text-center font-mono text-lg tracking-widest"
          maxlength={8}
          disabled={joining}
          autofocus
        />
      </div>

      {#if error}
        <p class="text-destructive text-sm">{error}</p>
      {/if}
    </div>

    <Dialog.Footer>
      <Button variant="outline" onclick={() => handleOpenChange(false)}>
        {m.common_button_cancel()}
      </Button>
      <Button onclick={handleJoin} disabled={joining || shareCode.length !== 8}>
        {#if joining}
          <Spinner class="mr-2 h-4 w-4" />
          {m.share_join_loading()}
        {:else}
          {m.share_join_button()}
        {/if}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
