<!-- Row.svelte -->
<script lang="ts">
    import { spacetimeClient } from "$lib/client/spacetime.svelte";
    import type { DocumentRow } from "$lib/module_bindings";
    import { tick } from "svelte";

    let { document_id, row_index, onNavigate, onDeleteRow, isFirstRow = false } = $props<{
        document_id: bigint;
        row_index: number;
        onNavigate?: (direction: 'up' | 'down', fromIndex: number, cursorPos?: number) => void;
        onDeleteRow?: (index: number) => void;
        isFirstRow?: boolean;
    }>();

    let document = $derived(
        spacetimeClient.documents.find(doc => doc.id === document_id) ||
        { title: "Loading...", content: "", rows: [] }
    );

    let row = $derived<DocumentRow>(
        document.rows[row_index] || { content: "", editor: undefined }
    );

    let textareaRef = $state<HTMLTextAreaElement>();
    let isEditing = $derived(row.editor && spacetimeClient.editorIsCurrentUser(row.editor));
    let isOtherEditing = $derived(row.editor && !spacetimeClient.editorIsCurrentUser(row.editor));

    // Track if we should show cursor/focus state
    let isFocused = $state(false);

    // Auto-resize textarea to fit content
    function adjustHeight() {
        if (!textareaRef) return;
        textareaRef.style.height = '0';
        textareaRef.style.height = textareaRef.scrollHeight + 'px';
    }

    function handleInput(event: Event) {
        const textarea = event.target as HTMLTextAreaElement;
        spacetimeClient.editRow(document_id, row_index, textarea.value);
        adjustHeight();
    }

    function handleKeyDown(event: KeyboardEvent) {
        const textarea = event.target as HTMLTextAreaElement;
        const cursorPos = textarea.selectionStart;
        const content = textarea.value;

        switch(event.key) {
            case 'Enter':
                event.preventDefault();
                spacetimeClient.splitRow(document_id, row_index, cursorPos);
                tick().then(() => {
                    onNavigate?.('down', row_index, 0);
                });
                break;

            case 'Backspace':
                if (cursorPos === 0 && row_index > 0) {
                    event.preventDefault();
                    const prevRow = document.rows[row_index - 1];
                    const prevLength = prevRow?.content.length || 0;

                    if (content === '') {
                        // Delete empty row and move to end of previous
                        onDeleteRow?.(row_index);
                        onNavigate?.('up', row_index, prevLength);
                    } else {
                        // Merge with previous row
                        spacetimeClient.mergeWithPreviousRow(document_id, row_index);
                        onNavigate?.('up', row_index, prevLength);
                    }
                }
                break;

            case 'ArrowUp':
                // Only intercept at the very beginning of the textarea
                if (cursorPos === 0) {
                    event.preventDefault();
                    const prevRow = document.rows[row_index - 1];
                    const prevLength = prevRow?.content.length || 0;
                    onNavigate?.('up', row_index, prevLength);
                }
                break;

            case 'ArrowDown':
                // Only intercept at the very end of the textarea
                if (cursorPos === content.length) {
                    event.preventDefault();
                    onNavigate?.('down', row_index, 0);
                }
                break;

            case 'Tab':
                event.preventDefault();
                const newContent =
                    content.substring(0, cursorPos) +
                    '  ' + // Use 2 spaces for tab
                    content.substring(cursorPos);
                textarea.value = newContent;
                textarea.setSelectionRange(cursorPos + 2, cursorPos + 2);
                spacetimeClient.editRow(document_id, row_index, newContent);
                break;
        }
    }

    function startEditing() {
        spacetimeClient.startEditingRow(document_id, row_index);
        isFocused = true;
        tick().then(() => {
            textareaRef?.focus();
            adjustHeight();
        });
    }

    function stopEditing() {
        spacetimeClient.stopEditingRow(document_id, row_index);
        isFocused = false;
    }

    function handleClick(event: MouseEvent) {
        event.preventDefault();
        startEditing();
    }

    // Public method for parent to call
    export function focus(position: 'start' | 'end' | number = 'start') {
        startEditing();
        tick().then(() => {
            if (!textareaRef) return;
            const pos = position === 'start' ? 0 :
                position === 'end' ? textareaRef.value.length :
                    position;
            textareaRef.setSelectionRange(pos, pos);
            adjustHeight();
        });
    }

    $effect(() => {
        if (isEditing && textareaRef) {
            adjustHeight();
        }
    });
</script>

<div class="group flex min-h-[1.5rem]">
    <!-- Line number gutter -->
    <div class="select-none text-xs w-10 text-right pr-3 pt-[2px]
                text-gray-500 dark:text-gray-600">
        {row_index + 1}
    </div>

    <!-- Editor content area -->
    <div class="flex-1 relative">
        {#if isEditing}
            <textarea
                    bind:this={textareaRef}
                    value={row.content}
                    oninput={handleInput}
                    onkeydown={handleKeyDown}
                    onblur={stopEditing}
                    class="w-full resize-none outline-none bg-transparent
                       font-mono text-sm leading-normal
                       text-gray-900 dark:text-gray-100
                       border-l-2 border-blue-500 dark:border-blue-400 pl-2 -ml-[2px]
                       overflow-hidden"
                    spellcheck="false"
            ></textarea>
        {:else if isOtherEditing}
            <div
                    class="font-mono text-sm leading-normal
                       whitespace-pre-wrap break-words
                       text-gray-900 dark:text-gray-100
                       border-l-2 border-yellow-400 dark:border-yellow-500
                       bg-yellow-50 dark:bg-yellow-900/20 pl-2 -ml-[2px]
                       min-h-[1.5rem]"
            >
                {row.content || '\u00A0'}
            </div>
        {:else}
            <button
                    onclick={handleClick}
                    class="w-full text-left font-mono text-sm leading-normal
                       whitespace-pre-wrap break-words
                       text-gray-900 dark:text-gray-100
                       hover:bg-gray-50 dark:hover:bg-gray-800/50
                       pl-2 cursor-text outline-none
                       min-h-[1.5rem] transition-colors duration-75"
            >
                {row.content || '\u00A0'}
            </button>
        {/if}
    </div>
</div>

<style>
    textarea {
        line-height: 1.5rem;
        padding-top: 0;
        padding-bottom: 0;
        min-height: 1.5rem;
    }
</style>