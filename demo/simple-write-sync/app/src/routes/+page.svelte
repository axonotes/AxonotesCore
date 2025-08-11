<script lang="ts">
    import {spacetimeClient} from "$lib/client/spacetime.svelte";

    let newDocTitle = $state("");
</script>

<div class="w-full">
    <div class="p-10 grid grid-cols-1 max-w-3xl m-auto">
        {#each spacetimeClient.documents as doc}
            <a class="p-4 border rounded-lg mb-4 flex flex-col -space-y-3 hover:border-primary-500 duration-200" href="/{doc.id}">
                <span class="text-xl font-bold">{doc.title}</span>
                <span class="text-sm text-gray-500">{doc.id}</span>
            </a>
        {/each}

        <div class="input-group grid-cols-[1fr_auto]">
            <input class="ig-input" type="text" placeholder="New Document Title" bind:value={newDocTitle} />
            <button class="ig-btn preset-filled" onclick={() => {
                newDocTitle = newDocTitle.trim();
                spacetimeClient.createDocument(newDocTitle);
                newDocTitle = ""; // Clear input after creation
            }}>
                Create Document
            </button>
        </div>
    </div>
</div>
