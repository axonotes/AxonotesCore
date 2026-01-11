<script lang="ts">
  import {onDestroy} from "svelte";
  import * as m from "$lib/paraglide/messages.js";
  import {Loader2, AlertCircle} from "@lucide/svelte";
  import {Button} from "$lib/components/ui/button";
  import EditorContent from "$lib/components/editor/EditorContent.svelte";
  import EditorActionBar from "$lib/components/editor/EditorActionBar.svelte";
  import {createEditorContext} from "$lib/components/editor/editorContext";

  interface Props {
    panelId?: string;
    params?: Record<string, unknown>;
  }

  let {panelId, params}: Props = $props();

  const docId = $derived((params?.docId as string) ?? null);
  const fileName = $derived((params?.fileName as string) ?? "Untitled");

  const editor = createEditorContext();
  const {loading, error} = editor;

  let loadedDocId: string | null = $state(null);

  $effect(() => {
    if (docId && docId !== loadedDocId) {
      loadedDocId = docId;
      editor.loadDocument(docId);
    }
  });

  onDestroy(() => {
    editor.cleanup();
  });

  function handleRetry() {
    if (docId) {
      loadedDocId = docId;
      editor.loadDocument(docId);
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
  {:else if $loading}
    <div class="flex flex-1 flex-col items-center justify-center gap-4">
      <Loader2 class="text-muted-foreground h-8 w-8 animate-spin" />
      <p class="text-muted-foreground text-sm">{m.editor_loading()}</p>
    </div>
  {:else if $error}
    <div class="flex flex-1 flex-col items-center justify-center gap-4 p-6">
      <AlertCircle class="text-destructive h-12 w-12" />
      <div class="text-center">
        <h3 class="text-lg font-medium">{m.editor_error_load()}</h3>
        <p class="text-muted-foreground mt-1 text-sm">{$error}</p>
      </div>
      <Button onclick={handleRetry} variant="outline">
        {m.editor_error_retry()}
      </Button>
    </div>
  {:else}
    <div class="flex-1 overflow-auto">
      <EditorActionBar {docId} docName={fileName} />
      <EditorContent />
    </div>
  {/if}
</div>
