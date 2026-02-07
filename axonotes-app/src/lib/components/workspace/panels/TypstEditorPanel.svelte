<!--
  TypstEditorPanel.svelte
  
  Panel shell for the Typst text editor. Handles:
  - Creating the TypstEditorContext (Svelte context for child components)
  - Loading the document (async) and passing initial text to the editor
  - Loading / error / empty states
  - Cleanup on destroy or document switch
-->
<script lang="ts">
  import {onDestroy} from "svelte";
  import * as m from "$lib/paraglide/messages.js";
  import {Loader2, AlertCircle} from "@lucide/svelte";
  import {Button} from "$lib/components/ui/button";
  import {createTypstEditorContext} from "$lib/components/typst-editor/typstEditorContext";
  import TypstEditor from "$lib/components/typst-editor/TypstEditor.svelte";

  interface Props {
    panelId?: string;
    params?: Record<string, unknown>;
  }

  let {panelId, params}: Props = $props();

  const docId = $derived((params?.docId as string) ?? null);
  const fileName = $derived((params?.fileName as string) ?? "Untitled");

  // Context is created synchronously at component init (required for setContext)
  const ctx = createTypstEditorContext();

  let loading = $state(false);
  let error = $state<string | null>(null);
  let initialText = $state<string | null>(null);
  let loadedDocId: string | null = null;

  // Load the document when docId changes.
  // Uses the same pattern as the reference EditorPanel: loadDocument() internally
  // calls cleanup() first, so we don't need separate cleanup logic here.
  $effect(() => {
    const currentDocId = docId;
    if (!currentDocId) return;
    if (currentDocId === loadedDocId) return;

    loadedDocId = currentDocId;
    loading = true;
    error = null;
    initialText = null;

    ctx
      .loadDocument(currentDocId)
      .then((text) => {
        // Guard: if docId changed while we were loading, ignore the result
        if (loadedDocId !== currentDocId) return;
        initialText = text;
        loading = false;
      })
      .catch((err) => {
        if (loadedDocId !== currentDocId) return;
        error = err instanceof Error ? err.message : String(err);
        loading = false;
      });
  });

  onDestroy(() => {
    ctx.cleanup();
  });

  function handleRetry() {
    if (docId) {
      // Force reload by resetting loadedDocId so the effect re-triggers
      loadedDocId = null;
      // Manually re-trigger since loadedDocId is not $state (not tracked)
      loading = true;
      error = null;
      initialText = null;

      ctx
        .loadDocument(docId)
        .then((text) => {
          if (loadedDocId !== null && loadedDocId !== docId) return;
          loadedDocId = docId;
          initialText = text;
          loading = false;
        })
        .catch((err) => {
          if (loadedDocId !== null && loadedDocId !== docId) return;
          loadedDocId = docId;
          error = err instanceof Error ? err.message : String(err);
          loading = false;
        });
    }
  }
</script>

<div class="flex h-full flex-col">
  {#if !docId}
    <div class="flex flex-1 items-center justify-center">
      <p class="text-muted-foreground text-center text-sm">
        {m.editor_placeholder()}
      </p>
    </div>
  {:else if loading}
    <div class="flex flex-1 flex-col items-center justify-center gap-4">
      <Loader2 class="text-muted-foreground h-8 w-8 animate-spin" />
      <p class="text-muted-foreground text-sm">{m.editor_loading()}</p>
    </div>
  {:else if error}
    <div class="flex flex-1 flex-col items-center justify-center gap-4 p-6">
      <AlertCircle class="text-destructive h-12 w-12" />
      <div class="text-center">
        <h3 class="text-lg font-medium">{m.editor_error_load()}</h3>
        <p class="text-muted-foreground mt-1 text-sm">{error}</p>
      </div>
      <Button onclick={handleRetry} variant="outline">
        {m.editor_error_retry()}
      </Button>
    </div>
  {:else if initialText !== null}
    <div class="flex-1 overflow-hidden">
      <TypstEditor {initialText} />
    </div>
  {/if}
</div>
