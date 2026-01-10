<script lang="ts">
  import {onMount, onDestroy, tick} from "svelte";
  import {
    Plus,
    Loader2,
    FilePlus,
    FolderPlus,
    Folder,
    Trash2,
    FolderInput,
  } from "@lucide/svelte";
  import {Button} from "$lib/components/ui/button";
  import * as ContextMenu from "$lib/components/ui/context-menu";
  import * as Dialog from "$lib/components/ui/dialog";
  import FileTreeNode from "./FileTreeNode.svelte";
  import * as m from "$lib/paraglide/messages.js";
  import {
    documents,
    documentsLoading,
    temporaryFolders,
    expandedFolders,
    selectedPaths,
    selectionFocus,
    loadDocuments,
    createDocument,
    deleteDocument,
    renameDocument,
    moveDocument,
    moveMultiple,
    createTemporaryFolder,
    removeTemporaryFolder,
    renameFolder,
    buildTree,
    flattenTreePaths,
    selectSingle,
    toggleSelection,
    selectRange,
    clearSelection,
    deleteMultiple,
    toggleFolderExpanded,
    setupDocumentListeners,
    cleanupDocumentListeners,
    type TreeNode,
  } from "$lib/stores/documents";
  import {addPanel, dockviewApi} from "$lib/stores/panels";
  import {get} from "svelte/store";

  interface Props {
    panelId?: string;
    params?: Record<string, unknown>;
  }

  let {panelId, params}: Props = $props();

  let creating = $state(false);

  // Rename state - stores just the filename WITHOUT extension
  let renameState = $state<{
    active: boolean;
    node: TreeNode | null;
    value: string; // Just the name part, without .doc
  }>({active: false, node: null, value: ""});

  // New folder input state
  let newFolderState = $state<{
    active: boolean;
    parentPath: string;
    value: string;
  }>({active: false, parentPath: "/", value: ""});

  // Build tree from documents and temporary folders
  let tree = $derived(buildTree($documents, $temporaryFolders));

  // Flat list of visible paths for range selection
  let flatPaths = $derived(flattenTreePaths(tree, $expandedFolders));

  // Currently hovered path (for arrow key starting point)
  let hoveredPath = $state<string | null>(null);

  // Delete confirmation state
  let deleteConfirm = $state<{
    open: boolean;
    paths: string[];
    moveToTrash: boolean;
  }>({open: false, paths: [], moveToTrash: true});

  // Current drop target path (folder path that's highlighted, or "/" for root)
  let dropTargetPath = $state<string | null>(null);

  // Handle hover for arrow key starting point
  function handleHover(path: string | null) {
    hoveredPath = path;
  }

  // Handle selection with modifiers
  function handleSelect(node: TreeNode, event: MouseEvent) {
    if (event.ctrlKey || event.metaKey) {
      // Ctrl/Cmd+click: toggle selection
      toggleSelection(node.path);
    } else if (event.shiftKey) {
      // Shift+click: range selection
      selectRange(flatPaths, node.path);
    } else {
      // Normal click: select single
      selectSingle(node.path);
    }
  }

  // Handle keyboard shortcuts
  function handleKeydown(event: KeyboardEvent) {
    // Disable all shortcuts when editing
    if (renameState.active || newFolderState.active) {
      return;
    }

    const selected = [...$selectedPaths];

    // Arrow navigation works even with no selection
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();

      // Determine starting point: current focus, hovered item, or default
      let currentPath = $selectionFocus;
      if (!currentPath && hoveredPath) {
        // Start from hovered item
        currentPath = hoveredPath;
      }

      const currentIndex = currentPath ? flatPaths.indexOf(currentPath) : -1;

      let nextIndex: number;
      if (currentIndex === -1) {
        // No selection and no hover - start at first or last item
        nextIndex = event.key === "ArrowDown" ? 0 : flatPaths.length - 1;
      } else if (event.key === "ArrowDown") {
        nextIndex =
          currentIndex < flatPaths.length - 1 ? currentIndex + 1 : currentIndex;
      } else {
        nextIndex = currentIndex > 0 ? currentIndex - 1 : 0;
      }

      const nextPath = flatPaths[nextIndex];
      if (!nextPath) return;

      if (event.shiftKey) {
        // Shift+Arrow: Select range from anchor to new focus
        selectRange(flatPaths, nextPath);
      } else {
        // Arrow: Move to single item (resets anchor)
        selectSingle(nextPath);
      }

      // Focus the newly selected item
      tick().then(() => {
        const button = document.querySelector(
          `[data-path="${CSS.escape(nextPath)}"]`
        ) as HTMLElement;
        button?.focus();
      });
      return;
    }

    // Other shortcuts require selection
    if (selected.length === 0) return;

    if (event.key === "F2" && selected.length === 1) {
      // F2: Rename (single selection only)
      event.preventDefault();
      const path = selected[0];
      const node = findNodeByPath(tree, path);
      if (node) {
        startRename(node);
      }
    } else if (event.key === "Delete" || event.key === "Backspace") {
      // Delete: Show confirmation modal
      event.preventDefault();
      deleteConfirm = {open: true, paths: selected, moveToTrash: true};
    } else if (event.key === "Enter" && selected.length === 1) {
      // Enter: Open file or toggle folder
      event.preventDefault();
      const path = selected[0];
      const node = findNodeByPath(tree, path);
      if (node?.type === "file" && node.docId) {
        openDocument(node.docId, node.name);
      } else if (node?.type === "folder") {
        toggleFolderExpanded(node.path);
      }
    } else if (event.key === "Escape") {
      // Escape: Clear selection
      clearSelection();
    }
  }

  // Find a node by path in the tree
  function findNodeByPath(nodes: TreeNode[], path: string): TreeNode | null {
    for (const node of nodes) {
      if (node.path === path) return node;
      if (node.children.length > 0) {
        const found = findNodeByPath(node.children, path);
        if (found) return found;
      }
    }
    return null;
  }

  // Handle delete confirmation
  async function confirmDelete() {
    if (deleteConfirm.moveToTrash) {
      // Move to trash folder instead of deleting
      await moveMultiple(deleteConfirm.paths, "/.trash");
    } else {
      // Permanently delete
      const result = await deleteMultiple(deleteConfirm.paths);
      if (result.errors.length > 0) {
        console.error("[Sidebar] Delete errors:", result.errors);
      }
    }
    deleteConfirm = {open: false, paths: [], moveToTrash: true};
  }

  function cancelDelete() {
    deleteConfirm = {open: false, paths: [], moveToTrash: true};
  }

  // Handle delete modal keyboard navigation
  function handleDeleteModalKeydown(event: KeyboardEvent) {
    if (event.key === "Tab") {
      event.preventDefault();
      deleteConfirm.moveToTrash = !deleteConfirm.moveToTrash;
    } else if (event.key === "Enter") {
      event.preventDefault();
      confirmDelete();
    } else if (event.key === "Escape") {
      event.preventDefault();
      cancelDelete();
    }
  }

  // Helper to extract name without .doc extension
  function getNameWithoutExtension(filename: string): string {
    if (filename.endsWith(".doc")) {
      return filename.slice(0, -4);
    }
    return filename;
  }

  // Helper to wait for a document to appear in the store
  async function waitForDocument(
    docId: string,
    maxWait = 2000
  ): Promise<{docId: string; path: string} | null> {
    const startTime = Date.now();
    while (Date.now() - startTime < maxWait) {
      const doc = $documents.find((d) => d.docId === docId);
      if (doc) return doc;
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    return null;
  }

  async function handleCreateDocument(inFolder: string = "/") {
    creating = true;
    try {
      // Create document directly in the target folder
      const docId = await createDocument(undefined, inFolder);

      // Wait for the document to appear in the store (populated by event listener)
      const doc = await waitForDocument(docId);
      if (!doc) return;

      // Wait for tree to update, then start rename
      await tick();
      const newTree = buildTree($documents, $temporaryFolders);
      const node = findNodeByPath(newTree, doc.path);
      if (node) {
        startRename(node);
      }
    } catch (error) {
      console.error("[Sidebar] Failed to create document:", error);
    } finally {
      creating = false;
    }
  }

  function handleFileClick(node: TreeNode) {
    if (!node.docId) return;
    openDocument(node.docId, node.name);
  }

  function openDocument(docId: string, fileName: string) {
    addPanel("editor", {
      id: `editor-${docId}`,
      params: {docId, fileName},
      title: fileName,
    });
  }

  // Rename handlers
  async function startRename(node: TreeNode) {
    renameState = {
      active: true,
      node,
      value:
        node.type === "file" ? getNameWithoutExtension(node.name) : node.name,
    };
    await tick();
    // Focus the input after render
    const input = document.querySelector(
      "input[data-rename-input]"
    ) as HTMLInputElement;
    input?.focus();
    input?.select();
  }

  async function submitRename() {
    if (!renameState.active || !renameState.node) return;
    const baseName = renameState.value.trim();
    if (!baseName) {
      cancelRename();
      return;
    }

    const node = renameState.node;

    if (node.type === "file") {
      // File rename - append .doc extension
      const newName = `${baseName}.doc`;

      if (newName === node.name) {
        cancelRename();
        return;
      }

      if (!node.docId) {
        cancelRename();
        return;
      }

      try {
        await renameDocument(node.docId, newName);
        // Update open panel title if exists
        const api = get(dockviewApi);
        const panel = api?.getPanel(`editor-${node.docId}`);
        if (panel) {
          panel.api.setTitle(newName);
        }
      } catch (error) {
        console.error("[Sidebar] Failed to rename file:", error);
      }
    } else {
      // Folder rename
      if (baseName === node.name) {
        cancelRename();
        return;
      }

      try {
        await renameFolder(node.path, baseName);
      } catch (error) {
        console.error("[Sidebar] Failed to rename folder:", error);
      }
    }
    cancelRename();
  }

  function cancelRename() {
    renameState = {active: false, node: null, value: ""};
  }

  function handleRenameKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      submitRename();
    } else if (event.key === "Escape") {
      cancelRename();
    }
  }

  function handleRenameInput(event: Event) {
    const input = event.target as HTMLInputElement;
    // Strip "/" characters from input
    if (input.value.includes("/")) {
      input.value = input.value.replace(/\//g, "");
      renameState.value = input.value;
    }
  }

  // New folder handlers
  async function startNewFolder(parentPath: string = "/") {
    newFolderState = {
      active: true,
      parentPath,
      value: "New Folder",
    };
    await tick();
    // Focus the input after render
    const input = document.querySelector(
      "input[data-newfolder-input]"
    ) as HTMLInputElement;
    input?.focus();
    input?.select();
  }

  function submitNewFolder() {
    const name = newFolderState.value.trim();
    if (!name) {
      cancelNewFolder();
      return;
    }

    const folderPath =
      newFolderState.parentPath === "/"
        ? `/${name}`
        : `${newFolderState.parentPath}/${name}`;

    createTemporaryFolder(folderPath);
    cancelNewFolder();
  }

  function cancelNewFolder() {
    newFolderState = {active: false, parentPath: "/", value: ""};
  }

  function handleNewFolderKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      submitNewFolder();
    } else if (event.key === "Escape") {
      cancelNewFolder();
    }
  }

  function handleNewFolderInput(event: Event) {
    const input = event.target as HTMLInputElement;
    // Strip "/" characters from input
    if (input.value.includes("/")) {
      input.value = input.value.replace(/\//g, "");
      newFolderState.value = input.value;
    }
  }

  // Delete handler - shows confirmation modal
  function handleDelete(node: TreeNode) {
    deleteConfirm = {open: true, paths: [node.path], moveToTrash: true};
  }

  // Drag and drop handler for multiple paths
  async function handleDrop(paths: string[], targetFolderPath: string) {
    dropTargetPath = null;
    try {
      await moveMultiple(paths, targetFolderPath);
    } catch (error) {
      console.error("[Sidebar] Failed to move items:", error);
    }
  }

  // Handle drag over - only update when target folder actually changes
  function handleDragTarget(targetPath: string) {
    if (dropTargetPath !== targetPath) {
      dropTargetPath = targetPath;
    }
  }

  // Handle drop on root (empty area)
  function handleRootDragOver(event: DragEvent) {
    event.preventDefault();
    event.dataTransfer!.dropEffect = "move";
    handleDragTarget("/");
  }

  function handleRootDragLeave(event: DragEvent) {
    // Only clear if leaving to outside the container
    const relatedTarget = event.relatedTarget as HTMLElement | null;
    const container = event.currentTarget as HTMLElement;
    if (!relatedTarget || !container.contains(relatedTarget)) {
      dropTargetPath = null;
    }
  }

  async function handleRootDrop(event: DragEvent) {
    event.preventDefault();
    dropTargetPath = null;
    const data = event.dataTransfer?.getData("application/json");
    if (data) {
      try {
        const paths = JSON.parse(data) as string[];
        await handleDrop(paths, "/");
      } catch {
        // Ignore parse errors
      }
    }
  }

  onMount(async () => {
    await setupDocumentListeners();
    await loadDocuments();
  });

  onDestroy(() => {
    cleanupDocumentListeners();
  });
</script>

<div class="relative flex h-full flex-col">
  <Button
    variant="ghost"
    size="sm"
    class="absolute top-2 right-2 z-10 h-6 w-6 p-0"
    onclick={() => handleCreateDocument()}
    disabled={creating || $documentsLoading}
    aria-label="Create new document"
  >
    {#if creating}
      <Loader2 class="h-4 w-4 animate-spin" />
    {:else}
      <Plus class="h-4 w-4" />
    {/if}
  </Button>

  <ContextMenu.Root>
    <ContextMenu.Trigger class="h-full overflow-auto">
      <div
        class="h-full rounded p-2 transition-colors focus:outline-none
          {dropTargetPath === '/'
          ? 'bg-primary/10 ring-primary cursor-move ring-2 ring-inset'
          : ''}"
        ondragover={handleRootDragOver}
        ondragleave={handleRootDragLeave}
        ondrop={handleRootDrop}
        onkeydown={handleKeydown}
        onclick={() => clearSelection()}
        role="tree"
        tabindex="0"
      >
        {#if $documentsLoading}
          <div class="flex items-center justify-center py-8">
            <Loader2 class="text-muted-foreground h-5 w-5 animate-spin" />
          </div>
        {:else if tree.length === 0 && !newFolderState.active}
          <p class="text-muted-foreground py-4 text-center text-xs">
            {m.sidebar_empty_state()}
          </p>
        {:else}
          <ul>
            {#if newFolderState.active && newFolderState.parentPath === "/"}
              <li>
                <div
                  class="bg-accent/50 flex items-center gap-2 rounded px-2 py-1"
                >
                  <span class="w-3 shrink-0"></span>
                  <Folder class="text-muted-foreground h-4 w-4 shrink-0" />
                  <input
                    type="text"
                    data-newfolder-input
                    bind:value={newFolderState.value}
                    onkeydown={handleNewFolderKeydown}
                    oninput={handleNewFolderInput}
                    onblur={submitNewFolder}
                    class="h-5 flex-1 bg-transparent text-sm focus:outline-none"
                  />
                </div>
              </li>
            {/if}
            {#each tree as node (node.path)}
              <FileTreeNode
                {node}
                onFileClick={handleFileClick}
                onCreateDocument={handleCreateDocument}
                onCreateFolder={startNewFolder}
                onRename={startRename}
                onDelete={handleDelete}
                onDrop={handleDrop}
                onSelect={handleSelect}
                onHover={handleHover}
                onDragTarget={handleDragTarget}
                {dropTargetPath}
                {newFolderState}
                onNewFolderInput={handleNewFolderInput}
                onNewFolderKeydown={handleNewFolderKeydown}
                onNewFolderBlur={submitNewFolder}
                onNewFolderValueChange={(v) => (newFolderState.value = v)}
                {renameState}
                onRenameInput={handleRenameInput}
                onRenameKeydown={handleRenameKeydown}
                onRenameBlur={submitRename}
                onRenameValueChange={(v) => (renameState.value = v)}
              />
            {/each}
          </ul>
        {/if}
      </div>
    </ContextMenu.Trigger>

    <ContextMenu.Content class="w-48">
      <ContextMenu.Item onclick={() => handleCreateDocument("/")}>
        <FilePlus class="mr-2 h-4 w-4" />
        {m.sidebar_new_document()}
      </ContextMenu.Item>

      <ContextMenu.Item onclick={() => startNewFolder("/")}>
        <FolderPlus class="mr-2 h-4 w-4" />
        {m.sidebar_new_folder()}
      </ContextMenu.Item>
    </ContextMenu.Content>
  </ContextMenu.Root>
</div>

<!-- Delete Confirmation Modal -->
<Dialog.Root bind:open={deleteConfirm.open}>
  <Dialog.Content class="sm:max-w-md" onkeydown={handleDeleteModalKeydown}>
    <Dialog.Header>
      <Dialog.Title>
        {#if deleteConfirm.paths.length === 1}
          {m.sidebar_delete_title_single()}
        {:else}
          {m.sidebar_delete_title_multi({count: deleteConfirm.paths.length})}
        {/if}
      </Dialog.Title>
      <Dialog.Description>
        {#if deleteConfirm.paths.length === 1}
          {m.sidebar_delete_confirm_single({
            name: deleteConfirm.paths[0].split("/").pop() ?? "",
          })}
        {:else}
          {m.sidebar_delete_confirm_multi({count: deleteConfirm.paths.length})}
        {/if}
      </Dialog.Description>
    </Dialog.Header>

    <div class="flex flex-col gap-3 py-4">
      <button
        onclick={() => {
          deleteConfirm.moveToTrash = true;
        }}
        class="flex items-center gap-3 rounded-lg border p-3 text-left transition-colors duration-150
          {deleteConfirm.moveToTrash
          ? 'border-primary bg-primary/10'
          : 'border-border hover:bg-accent'}"
      >
        <FolderInput class="text-muted-foreground h-5 w-5" />
        <div>
          <div class="font-medium">{m.sidebar_move_to_trash()}</div>
          <div class="text-muted-foreground text-sm">
            {m.sidebar_move_to_trash_desc()}
          </div>
        </div>
      </button>

      <button
        onclick={() => {
          deleteConfirm.moveToTrash = false;
        }}
        class="flex items-center gap-3 rounded-lg border p-3 text-left transition-colors duration-150
          {!deleteConfirm.moveToTrash
          ? 'border-destructive bg-destructive/10'
          : 'border-border hover:bg-accent'}"
      >
        <Trash2 class="text-destructive h-5 w-5" />
        <div>
          <div class="font-medium">{m.sidebar_delete_permanently()}</div>
          <div class="text-muted-foreground text-sm">
            {m.sidebar_delete_permanently_desc()}
          </div>
        </div>
      </button>
    </div>

    <Dialog.Footer>
      <Button variant="outline" onclick={cancelDelete}
        >{m.common_button_cancel()}</Button
      >
      <Button
        variant={deleteConfirm.moveToTrash ? "default" : "destructive"}
        onclick={confirmDelete}
      >
        {deleteConfirm.moveToTrash
          ? m.sidebar_move_to_trash()
          : m.sidebar_delete_permanently()}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
