<script lang="ts">
  import {Folder, FileText, ChevronRight} from "@lucide/svelte";

  interface Props {
    panelId?: string;
    params?: Record<string, unknown>;
  }

  let {panelId, params}: Props = $props();

  // Example file tree structure
  const files = [
    {name: "Documents", type: "folder", children: ["Notes.md", "Todo.md"]},
    {name: "Projects", type: "folder", children: ["Project A", "Project B"]},
    {name: "Quick Notes.md", type: "file"},
    {name: "Ideas.md", type: "file"},
  ];
</script>

<div class="flex h-full flex-col">
  <div class="border-b p-2">
    <h3 class="text-sm font-medium">Files</h3>
  </div>
  <div class="flex-1 overflow-auto p-2">
    <ul class="space-y-1">
      {#each files as item (item.name)}
        <li>
          <button
            class="hover:bg-accent flex w-full items-center gap-2 rounded px-2 py-1 text-left text-sm"
          >
            {#if item.type === "folder"}
              <ChevronRight class="text-muted-foreground h-3 w-3" />
              <Folder class="text-muted-foreground h-4 w-4" />
            {:else}
              <span class="w-3"></span>
              <FileText class="text-muted-foreground h-4 w-4" />
            {/if}
            <span>{item.name}</span>
          </button>
        </li>
      {/each}
    </ul>
  </div>
</div>
