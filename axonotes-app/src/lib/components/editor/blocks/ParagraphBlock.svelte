<script lang="ts">
  import {tick} from "svelte";
  import {get} from "svelte/store";
  import {getEditorContext} from "../editorContext";
  import {createParagraphBlock} from "$lib/services/block";

  interface Props {
    blockId: number;
  }

  let {blockId}: Props = $props();

  const editor = getEditorContext();
  const {blocks, focusedBlockId, lockedBlockId, blockOrder} = editor;

  let block = $derived($blocks.get(blockId));
  let text = $derived(block?.content.text ?? "");
  let formatting = $derived(block?.content.formatting ?? []);

  let isOwnFocused = $derived($focusedBlockId === blockId);
  let isOwnLocked = $derived($lockedBlockId === blockId);
  let isReadOnly = $derived.by(() => {
    const liveInfo = block?.liveInfo;
    return liveInfo?.state === "locked" && !isOwnLocked;
  });

  let displayText = $derived.by(() => {
    const liveInfo = block?.liveInfo;
    if (liveInfo?.state === "locked" && liveInfo.content) {
      return liveInfo.content.text ?? "";
    }
    return text;
  });

  let contentRef: HTMLDivElement | null = $state(null);
  let hasTypedSinceLastFocus = $state(false);
  let isCreatingBlock = $state(false);

  function handleFocus() {
    const currentFocusedId = get(focusedBlockId);
    if (currentFocusedId !== blockId) {
      editor.focusBlock(blockId);
    }
    hasTypedSinceLastFocus = false;
  }

  function handleBlur() {
    hasTypedSinceLastFocus = false;
  }

  function textToHtml(text: string): string {
    return escapeHtml(text).replace(/\n/g, "<br>");
  }

  function setContent(element: HTMLDivElement, text: string): void {
    element.innerHTML = textToHtml(text);
  }

  async function handleInput(event: Event) {
    if (isReadOnly) {
      if (contentRef) {
        setContent(contentRef, displayText);
      }
      return;
    }

    const target = event.target as HTMLDivElement;
    const newText = target.innerText ?? "";

    const currentLock = get(lockedBlockId);
    if (!hasTypedSinceLastFocus && currentLock !== blockId) {
      hasTypedSinceLastFocus = true;
      const gotLock = await editor.lockBlock(blockId);
      if (!gotLock) {
        console.warn(
          "[ParagraphBlock] Failed to acquire lock for block:",
          blockId
        );
        if (contentRef) {
          setContent(contentRef, displayText);
        }
        return;
      }
    }

    editor.updateBlockContent(blockId, {text: newText});
  }

  async function handleKeyDown(event: KeyboardEvent) {
    // Enter: create new block
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();

      if (isReadOnly || isCreatingBlock) return;
      isCreatingBlock = true;

      try {
        const selection = window.getSelection();
        const cursorPos = selection?.focusOffset ?? text.length;
        const currentText = contentRef?.innerText ?? "";

        const beforeCursor = currentText.slice(0, cursorPos);
        const afterCursor = currentText.slice(cursorPos);

        if (beforeCursor !== text) {
          editor.updateBlockContent(blockId, {text: beforeCursor});
        }

        const newBlock = await createParagraphBlock(0, afterCursor);
        const newId = await editor.createBlockAfter(blockId, newBlock);

        if (newId) {
          await tick();

          const newElement = document.querySelector(
            `[data-block-id="${newId}"]`
          ) as HTMLElement;

          if (newElement) {
            newElement.focus();
            const range = document.createRange();
            range.selectNodeContents(newElement);
            range.collapse(true);
            const sel = window.getSelection();
            sel?.removeAllRanges();
            sel?.addRange(range);
          }
        }
      } finally {
        isCreatingBlock = false;
      }
      return;
    }

    // Shift+Enter: soft line break (browser default)
    if (event.key === "Enter" && event.shiftKey) {
      return;
    }

    // Escape: blur and release lock
    if (event.key === "Escape") {
      event.preventDefault();
      contentRef?.blur();
      editor.blurBlock();
      return;
    }

    // Backspace at start: delete or merge with previous
    if (event.key === "Backspace") {
      const selection = window.getSelection();
      const cursorPos = selection?.focusOffset ?? 0;

      if (cursorPos === 0 && selection?.isCollapsed) {
        const currentText = contentRef?.innerText ?? "";
        const prevId = editor.getPrevBlockId(blockId);
        const nextId = editor.getNextBlockId(blockId);
        const blockCount = get(blockOrder).length;

        if (blockCount <= 1) {
          return;
        }

        event.preventDefault();

        // Empty block: delete it
        if (currentText === "") {
          await editor.deleteBlock(blockId);
          await tick();

          const targetId = prevId ?? nextId;
          if (targetId !== null) {
            const targetElement = document.querySelector(
              `[data-block-id="${targetId}"]`
            ) as HTMLElement;

            if (targetElement) {
              targetElement.focus();
              const range = document.createRange();
              range.selectNodeContents(targetElement);
              range.collapse(prevId === null);
              const sel = window.getSelection();
              sel?.removeAllRanges();
              sel?.addRange(range);
            }

            editor.focusBlock(targetId);
          }
          return;
        }

        // Has content: merge with previous
        if (prevId !== null) {
          const prevBlock = get(blocks).get(prevId);
          const prevText = prevBlock?.content.text ?? "";

          editor.updateBlockContent(prevId, {text: prevText + currentText});
          await editor.deleteBlock(blockId);
          await tick();

          const prevElement = document.querySelector(
            `[data-block-id="${prevId}"]`
          ) as HTMLElement;

          if (prevElement) {
            prevElement.focus();
            const range = document.createRange();
            const textNode = prevElement.firstChild;
            if (textNode) {
              range.setStart(textNode, prevText.length);
              range.collapse(true);
              const sel = window.getSelection();
              sel?.removeAllRanges();
              sel?.addRange(range);
            }
          }

          editor.focusBlock(prevId);
        }
      }
      return;
    }

    // TODO: formatting shortcuts (Cmd+B, Cmd+I, etc.)
  }

  // TODO: render FormatSpan array to HTML
  function renderContent(): string {
    return escapeHtml(displayText);
  }

  function escapeHtml(str: string): string {
    return str
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  }

  // Sync DOM with state for real-time collaboration
  $effect(() => {
    if (contentRef && !isOwnLocked) {
      const currentContent = contentRef.innerText ?? "";
      if (currentContent !== displayText) {
        setContent(contentRef, displayText);
      }
    }
  });
</script>

<div
  bind:this={contentRef}
  data-block-id={blockId}
  contenteditable={!isReadOnly}
  class="text-foreground min-h-[1.5em] leading-relaxed outline-none"
  class:text-muted-foreground={isReadOnly}
  role="textbox"
  tabindex="0"
  aria-multiline="true"
  aria-readonly={isReadOnly}
  onfocus={handleFocus}
  onblur={handleBlur}
  oninput={handleInput}
  onkeydown={handleKeyDown}
></div>
