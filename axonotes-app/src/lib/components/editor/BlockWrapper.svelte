<script lang="ts">
  import {GripVertical} from "@lucide/svelte";
  import {getEditorContext, USER_COLORS} from "./editorContext";
  import ParagraphBlock from "./blocks/ParagraphBlock.svelte";

  interface Props {
    blockId: number;
  }

  let {blockId}: Props = $props();

  const editor = getEditorContext();
  const {blocks, focusedBlockId, lockedBlockId} = editor;

  let block = $derived($blocks.get(blockId));
  let blockContent = $derived(block?.content);
  let liveInfo = $derived(block?.liveInfo);

  let isOwnFocused = $derived($focusedBlockId === blockId);
  let isOwnLocked = $derived($lockedBlockId === blockId);
  let isOtherLocked = $derived(liveInfo?.state === "locked" && !isOwnLocked);
  let isOtherFocused = $derived(liveInfo?.state === "focused" && !isOwnFocused);

  let ownColor = USER_COLORS[0];
  let otherColor = $derived(liveInfo?.color ?? USER_COLORS[1]);

  let showBorder = $derived(
    isOwnFocused || isOwnLocked || isOtherFocused || isOtherLocked
  );
  let isLockedState = $derived(isOwnLocked || isOtherLocked);

  let borderColor = $derived.by(() => {
    if (isOwnLocked || isOwnFocused) return ownColor;
    if (isOtherLocked || isOtherFocused) return otherColor;
    return "transparent";
  });

  let showNameTag = $derived(isOtherLocked || isOtherFocused);
  let nameTagUser = $derived(liveInfo?.username ?? "");

  function handleClick() {
    if (!isOwnFocused) {
      editor.focusBlock(blockId);
    }
  }

  let showDragHandle = $state(false);
</script>

<div
  class="group relative transition-colors duration-150"
  class:rounded-lg={showBorder}
  style={showBorder
    ? `border: ${isLockedState ? "3px solid" : "2px dotted"} ${borderColor};`
    : ""}
  role="article"
  onmouseenter={() => (showDragHandle = true)}
  onmouseleave={() => (showDragHandle = false)}
  onclick={handleClick}
>
  {#if showNameTag}
    <div
      class="absolute -top-3 left-2 z-10 rounded px-2 py-0.5 text-xs text-white"
      style="background-color: {otherColor};"
    >
      {nameTagUser}
    </div>
  {/if}

  <div
    class="absolute top-1/2 -left-6 flex h-6 w-6 -translate-y-1/2 cursor-grab items-center justify-center rounded transition-opacity duration-150"
    class:opacity-0={!showDragHandle && !isOwnFocused}
    class:opacity-50={showDragHandle && !isOwnFocused}
    class:opacity-100={isOwnFocused}
  >
    <GripVertical class="text-muted-foreground h-4 w-4" />
  </div>

  <div class="min-h-[1.5em] px-1 py-0.5">
    {#if blockContent}
      {#if blockContent.block_type === "paragraph"}
        <ParagraphBlock {blockId} />
      {:else if blockContent.block_type === "heading"}
        <!-- TODO: HeadingBlock -->
        <div class="text-foreground font-semibold">
          {blockContent.text ?? ""}
        </div>
      {:else}
        <div class="text-muted-foreground text-sm italic">
          Unknown block type: {blockContent.block_type}
        </div>
      {/if}
    {/if}
  </div>
</div>
