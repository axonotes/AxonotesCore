<script lang="ts">
  import {onMount, onDestroy} from "svelte";
  import {slide} from "svelte/transition";
  import {Copy, Check, Pencil, Eye, Loader2, History} from "@lucide/svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import {Button} from "$lib/components/ui/button";
  import {Checkbox} from "$lib/components/ui/checkbox";
  import {ShareService, type ShareCodeReadyPayload} from "$lib/services/share";
  import CollaboratorList from "./CollaboratorList.svelte";
  import * as m from "$lib/paraglide/messages.js";
  import type {UnlistenFn} from "@tauri-apps/api/event";

  interface Props {
    open: boolean;
    docId: string;
    docName?: string;
    onOpenChange?: (open: boolean) => void;
  }

  let {open = $bindable(false), docId, docName, onOpenChange}: Props = $props();

  // Share creation state
  let role = $state<"editor" | "reader">("editor");
  let fullHistory = $state(false);
  let shareCode = $state<string | null>(null);
  let generating = $state(false);
  let closing = $state(false);
  let copied = $state(false);
  let error = $state<string | null>(null);

  let unlisteners: UnlistenFn[] = [];

  // Has an active share session for this document
  let hasActiveShare = $derived(shareCode !== null);

  async function handleGenerateCode() {
    generating = true;
    error = null;
    try {
      await ShareService.createShare(docId, role, fullHistory);
      // Share code will be set via event listener
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      generating = false;
    }
  }

  async function handleCloseShare() {
    closing = true;
    error = null;
    try {
      await ShareService.closeShare(docId);
      shareCode = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      closing = false;
    }
  }

  async function copyToClipboard() {
    if (!shareCode) return;
    try {
      await navigator.clipboard.writeText(shareCode);
      copied = true;
      setTimeout(() => {
        copied = false;
      }, 2000);
    } catch (e) {
      console.error("Failed to copy:", e);
    }
  }

  function handleShareCodeReady(payload: ShareCodeReadyPayload) {
    if (payload.docId === docId) {
      shareCode = payload.shareCode;
      generating = false;
    }
  }

  function handleShareClosed(payload: {docId: string; shareCode: string}) {
    if (payload.docId === docId) {
      shareCode = null;
    }
  }

  function handleOpenChange(newOpen: boolean) {
    open = newOpen;
    onOpenChange?.(newOpen);
    if (!newOpen) {
      // Reset state when closing
      error = null;
      copied = false;
    }
  }

  onMount(async () => {
    unlisteners.push(
      await ShareService.onShareCodeReady(handleShareCodeReady),
      await ShareService.onShareClosed(handleShareClosed)
    );
  });

  onDestroy(() => {
    unlisteners.forEach((unlisten) => unlisten());
  });
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
  <Dialog.Content class="max-w-sm">
    <Dialog.Header>
      <Dialog.Title>{m.share_dialog_title()}</Dialog.Title>
      {#if docName}
        <Dialog.Description class="truncate">
          {docName}
        </Dialog.Description>
      {/if}
    </Dialog.Header>

    <div class="space-y-5 py-4">
      {#if hasActiveShare}
        <!-- Active Share Section -->
        <div class="space-y-4">
          <!-- Share Code Display -->
          <div class="bg-muted/50 rounded-lg border p-4">
            <div class="flex items-center justify-between">
              <code class="font-mono text-lg tracking-widest">{shareCode}</code>
              <Button
                variant="ghost"
                size="icon"
                onclick={copyToClipboard}
                aria-label={m.common_action_copy()}
                class="h-8 w-8 shrink-0"
              >
                {#if copied}
                  <Check class="h-4 w-4 text-green-600 dark:text-green-500" />
                {:else}
                  <Copy class="h-4 w-4" />
                {/if}
              </Button>
            </div>
            <p class="text-muted-foreground mt-2 text-xs">
              {m.share_code_hint()}
            </p>
          </div>

          <Button
            variant="outline"
            class="border-destructive/50 text-destructive hover:bg-destructive hover:text-destructive-foreground w-full"
            onclick={handleCloseShare}
            disabled={closing}
          >
            {#if closing}
              <Loader2 class="mr-2 h-4 w-4 animate-spin" />
            {/if}
            {m.share_close_button()}
          </Button>
        </div>
      {:else}
        <!-- Create Share Section -->
        <div class="space-y-4">
          <!-- Role Selection -->
          <div class="space-y-3">
            <button
              class="hover:bg-accent w-full rounded-lg border p-3 text-left transition-colors {role ===
              'editor'
                ? 'border-primary bg-accent'
                : ''}"
              onclick={() => (role = "editor")}
              type="button"
            >
              <div class="flex items-center gap-3">
                <Pencil class="text-muted-foreground h-4 w-4" />
                <div>
                  <p class="text-sm font-medium">{m.share_role_editor()}</p>
                  <p class="text-muted-foreground text-xs">
                    {m.share_role_editor_description()}
                  </p>
                </div>
              </div>
            </button>

            <button
              class="hover:bg-accent w-full rounded-lg border p-3 text-left transition-colors {role ===
              'reader'
                ? 'border-primary bg-accent'
                : ''}"
              onclick={() => (role = "reader")}
              type="button"
            >
              <div class="flex items-center gap-3">
                <Eye class="text-muted-foreground h-4 w-4" />
                <div>
                  <p class="text-sm font-medium">{m.share_role_reader()}</p>
                  <p class="text-muted-foreground text-xs">
                    {m.share_role_reader_description()}
                  </p>
                </div>
              </div>
            </button>
          </div>

          <!-- Full History Option -->
          <label
            class="hover:bg-accent/50 flex cursor-pointer items-center gap-3 rounded-lg border p-3 transition-colors"
          >
            <Checkbox id="full-history" bind:checked={fullHistory} />
            <div class="flex items-center gap-3">
              <History class="text-muted-foreground h-4 w-4" />
              <div>
                <p class="text-sm font-medium">
                  {m.share_create_history_label()}
                </p>
                <p class="text-muted-foreground text-xs">
                  {m.share_create_history_description()}
                </p>
              </div>
            </div>
          </label>

          {#if error}
            <p
              class="text-destructive text-sm"
              transition:slide={{duration: 150}}
            >
              {error}
            </p>
          {/if}
        </div>
      {/if}

      <!-- Collaborators Section -->
      <CollaboratorList {docId} />
    </div>

    {#if !hasActiveShare}
      <Dialog.Footer>
        <Button
          class="w-full"
          onclick={handleGenerateCode}
          disabled={generating}
        >
          {#if generating}
            <Loader2 class="mr-2 h-4 w-4 animate-spin" />
          {/if}
          {m.share_create_button()}
        </Button>
      </Dialog.Footer>
    {/if}
  </Dialog.Content>
</Dialog.Root>
