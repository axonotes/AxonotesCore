<!-- +page.svelte -->
<script lang="ts">
    import { page } from "$app/state";
    import { spacetimeClient } from "$lib/client/spacetime.svelte";
    import Row from "$lib/components/Row.svelte";
    import { tick } from "svelte";

    let document_id = BigInt(page.params.id);
    let rowRefs = $state<Row[]>([]);

    let document = $derived(
        spacetimeClient.documents.find(doc => doc.id === document_id) ||
        { title: "Loading...", content: "", rows: [] }
    );

    // Navigation handler with cursor position support
    async function handleNavigate(direction: 'up' | 'down', fromIndex: number, cursorPos?: number) {
        await tick();

        if (direction === 'up' && fromIndex > 0) {
            const targetPos = cursorPos !== undefined ? cursorPos : 'end';
            rowRefs[fromIndex - 1]?.focus(targetPos);
        } else if (direction === 'down') {
            if (fromIndex < document.rows.length - 1) {
                const targetPos = cursorPos !== undefined ? cursorPos : 'start';
                rowRefs[fromIndex + 1]?.focus(targetPos);
            } else {
                // At last row, create new row
                spacetimeClient.addRow(document_id);
                await tick();
                rowRefs[rowRefs.length - 1]?.focus('start');
            }
        }
    }

    // Delete row handler
    function handleDeleteRow(index: number) {
        if (document.rows.length > 1) {
            spacetimeClient.deleteRow(document_id, index);
        }
    }

    // Global keyboard shortcuts
    function handleDocumentKeyDown(event: KeyboardEvent) {
        if (event.metaKey || event.ctrlKey) {
            switch(event.key) {
                case 's':
                    event.preventDefault();
                    // Document is auto-saved
                    console.log('Document auto-saved');
                    break;
            }
        }
    }

    // Click on empty space to focus last row or create new
    function handleEmptyClick() {
        if (document.rows.length === 0) {
            spacetimeClient.addRow(document_id);
            tick().then(() => rowRefs[0]?.focus('start'));
        } else {
            rowRefs[document.rows.length - 1]?.focus('end');
        }
    }

    // Ensure at least one row exists
    $effect(() => {
        if (document.rows.length === 0) {
            spacetimeClient.addRow(document_id);
        }
    });
</script>

<div class="min-h-screen bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100"
     onkeydown={handleDocumentKeyDown}>
    <div class="max-w-4xl mx-auto">
        <!-- Document header -->
        <div class="px-14 py-6 border-b border-gray-200 dark:border-gray-800">
            <input
                    value={document.title}
                    oninput={(e) => spacetimeClient.updateTitle(document_id, e.currentTarget.value)}
                    class="text-2xl font-semibold w-full outline-none bg-transparent
                       placeholder-gray-400 dark:placeholder-gray-600
                       focus:placeholder-gray-500 dark:focus:placeholder-gray-500"
                    placeholder="Untitled"
            >
        </div>

        <!-- Document content -->
        <div class="py-4">
            {#each document.rows as _row, index (index)}
                <Row
                        bind:this={rowRefs[index]}
                        {document_id}
                        row_index={index}
                        isFirstRow={index === 0}
                        onNavigate={handleNavigate}
                        onDeleteRow={handleDeleteRow}
                />
            {/each}
        </div>

        <!-- Click area for new lines -->
        <button
                onclick={handleEmptyClick}
                class="w-full h-96 text-left px-14 outline-none"
                aria-label="Click to add content"
        >
            <!-- Empty clickable area -->
        </button>
    </div>
</div>

<style>
    /* Ensure smooth cursor */
    :global(*) {
        caret-color: rgb(59 130 246);
    }

    :global(.dark *) {
        caret-color: rgb(96 165 250);
    }

    /* Remove any focus outlines globally for this editor */
    :global(.editor-container *:focus) {
        outline: none;
    }
</style>