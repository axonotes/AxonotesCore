<script lang="ts">
  import * as m from "$lib/paraglide/messages.js";
  import {FileText} from "@lucide/svelte";
  import {Button} from "$lib/components/ui/button";
  import {getEditorContext} from "./editorContext";
  import {createParagraphBlock} from "$lib/services/block";
  import BlockWrapper from "./BlockWrapper.svelte";

  const editor = getEditorContext();
  const {hasContent, orderedBlocks} = editor;

  function handleContainerClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      editor.blurBlock();
    }
  }

  function handleContainerKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      editor.blurBlock();
    }
  }

  async function handleCreateFirstBlock() {
    const block = await createParagraphBlock(0, "");
    const newId = await editor.createBlockAfter(null, block);
    if (newId) {
      editor.focusBlock(newId);
    }
  }
</script>

<div
  class="flex h-full flex-col p-6"
  role="region"
  aria-label="Document editor"
  onclick={handleContainerClick}
  onkeydown={handleContainerKeyDown}
>
  {#if $hasContent}
    {#each $orderedBlocks as block (block.id)}
      <BlockWrapper blockId={block.id} />
    {/each}
  {:else}
    <div
      class="flex flex-1 flex-col items-center justify-center py-12 text-center"
    >
      <FileText class="text-muted-foreground mb-4 h-12 w-12" />
      <h3 class="mb-2 text-lg font-medium">{m.editor_empty_title()}</h3>
      <p class="text-muted-foreground mb-4 text-sm">
        {m.editor_empty_description()}
      </p>
      <Button onclick={handleCreateFirstBlock}>
        {m.editor_slash_paragraph()}
      </Button>
    </div>
  {/if}
</div>
