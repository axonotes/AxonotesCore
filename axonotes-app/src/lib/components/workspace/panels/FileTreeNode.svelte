<script lang="ts">
  import {
    Folder,
    FileText,
    ChevronRight,
    FilePlus,
    FolderPlus,
    Pencil,
    Trash2,
    Share2,
  } from "@lucide/svelte";
  import * as ContextMenu from "$lib/components/ui/context-menu";
  import * as m from "$lib/paraglide/messages.js";
  import FileTreeNode from "./FileTreeNode.svelte";
  import {
    expandedFolders,
    toggleFolderExpanded,
    setFolderExpanded,
    selectedPaths,
    selectSingle,
    toggleSelection,
    type TreeNode,
  } from "$lib/stores/documents";

  interface NewFolderState {
    active: boolean;
    parentPath: string;
    value: string;
  }

  interface RenameState {
    active: boolean;
    node: TreeNode | null;
    value: string;
  }

  interface Props {
    node: TreeNode;
    depth?: number;
    onFileClick: (node: TreeNode) => void;
    onCreateDocument: (folderPath: string) => void;
    onCreateFolder: (parentPath: string) => void;
    onRename: (node: TreeNode) => void;
    onDelete: (node: TreeNode) => void;
    onShare?: (node: TreeNode) => void;
    onDrop: (paths: string[], targetFolderPath: string) => void;
    onSelect: (node: TreeNode, event: MouseEvent) => void;
    onHover: (path: string | null) => void;
    onDragTarget: (targetPath: string) => void;
    dropTargetPath: string | null;
    newFolderState?: NewFolderState;
    onNewFolderInput?: (event: Event) => void;
    onNewFolderKeydown?: (event: KeyboardEvent) => void;
    onNewFolderBlur?: () => void;
    onNewFolderValueChange?: (value: string) => void;
    renameState?: RenameState;
    onRenameInput?: (event: Event) => void;
    onRenameKeydown?: (event: KeyboardEvent) => void;
    onRenameBlur?: () => void;
    onRenameValueChange?: (value: string) => void;
  }

  let {
    node,
    depth = 0,
    onFileClick,
    onCreateDocument,
    onCreateFolder,
    onRename,
    onDelete,
    onShare,
    onDrop,
    onSelect,
    onHover,
    onDragTarget,
    dropTargetPath,
    newFolderState,
    onNewFolderInput,
    onNewFolderKeydown,
    onNewFolderBlur,
    onNewFolderValueChange,
    renameState,
    onRenameInput,
    onRenameKeydown,
    onRenameBlur,
    onRenameValueChange,
  }: Props = $props();

  // Use persisted expanded state from store
  let expanded = $derived($expandedFolders.has(node.path));
  let isSelected = $derived($selectedPaths.has(node.path));
  let isRenaming = $derived(
    renameState?.active && renameState?.node?.path === node.path
  );

  // Auto-expand when creating a subfolder inside this folder
  $effect(() => {
    if (newFolderState?.active && newFolderState.parentPath === node.path) {
      setFolderExpanded(node.path, true);
    }
  });

  function handleClick(event: MouseEvent) {
    // Stop propagation to prevent container's clearSelection from firing
    event.stopPropagation();

    // Handle selection
    onSelect(node, event);

    // Double-click on file opens it
    if (node.type === "file" && event.detail === 2) {
      onFileClick(node);
    }
    // Single click on folder toggles expand (if not ctrl/shift clicking)
    else if (
      node.type === "folder" &&
      !event.ctrlKey &&
      !event.metaKey &&
      !event.shiftKey
    ) {
      toggleFolderExpanded(node.path);
    }
  }

  // Get the target folder for "New Document" / "New Folder" actions
  function getTargetFolder(): string {
    return node.type === "folder" ? node.path : getParentPath(node.path);
  }

  function getParentPath(path: string): string {
    const parts = path.split("/").filter(Boolean);
    parts.pop();
    return parts.length === 0 ? "/" : "/" + parts.join("/");
  }

  // Drag source (files and folders)
  function handleDragStart(event: DragEvent) {
    // If this item is selected, drag all selected items
    // Otherwise, select just this item and drag it
    let pathsToDrag: string[];

    if (isSelected) {
      pathsToDrag = [...$selectedPaths];
    } else {
      selectSingle(node.path);
      pathsToDrag = [node.path];
    }

    event.dataTransfer?.setData(
      "application/json",
      JSON.stringify(pathsToDrag)
    );
    event.dataTransfer!.effectAllowed = "move";
  }

  // Drop target - folders accept drops directly, documents highlight their parent folder
  function handleDragOver(event: DragEvent) {
    // Determine the target folder path
    const targetFolderPath =
      node.type === "folder" ? node.path : getParentPath(node.path);

    // Don't allow dropping on itself or its children
    const data = event.dataTransfer?.getData("application/json");
    if (data) {
      try {
        const paths = JSON.parse(data) as string[];
        if (
          paths.some(
            (p) =>
              targetFolderPath === p || targetFolderPath.startsWith(p + "/")
          )
        ) {
          return;
        }
      } catch {
        // Ignore parse errors
      }
    }
    event.preventDefault();
    event.stopPropagation(); // Prevent root from highlighting
    event.dataTransfer!.dropEffect = "move";
    onDragTarget(targetFolderPath);
  }

  function handleDragLeave() {
    // Don't clear here - let the container handle clearing when leaving the tree entirely
  }

  function handleDropEvent(event: DragEvent) {
    event.preventDefault();
    event.stopPropagation(); // Prevent bubbling to root container

    // Determine target folder - for documents, drop into their parent folder
    const targetFolderPath =
      node.type === "folder" ? node.path : getParentPath(node.path);

    const data = event.dataTransfer?.getData("application/json");
    if (data) {
      try {
        const paths = JSON.parse(data) as string[];
        // Don't drop on self or children
        if (
          paths.some(
            (p) =>
              targetFolderPath === p || targetFolderPath.startsWith(p + "/")
          )
        ) {
          return;
        }
        onDrop(paths, targetFolderPath);
      } catch {
        // Ignore parse errors
      }
    }
  }
</script>

<li>
  <div
    class="pb-0.5"
    role="treeitem"
    aria-selected={isSelected}
    tabindex="-1"
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDropEvent}
  >
    {#if isRenaming}
      <div
        class="bg-accent/50 flex items-center gap-2 rounded px-2 py-1"
        style="padding-left: {depth * 12 + 8}px"
      >
        {#if node.type === "folder"}
          <ChevronRight
            class="text-muted-foreground h-3 w-3 shrink-0 transition-transform {expanded
              ? 'rotate-90'
              : ''}"
          />
          <Folder class="text-muted-foreground h-4 w-4 shrink-0" />
        {:else}
          <span class="w-3 shrink-0"></span>
          <FileText class="text-muted-foreground h-4 w-4 shrink-0" />
        {/if}
        <input
          type="text"
          data-rename-input
          value={renameState?.value ?? ""}
          oninput={(e) => {
            onRenameInput?.(e);
            onRenameValueChange?.((e.target as HTMLInputElement).value);
          }}
          onkeydown={onRenameKeydown}
          onblur={onRenameBlur}
          class="h-5 min-w-0 flex-1 bg-transparent text-sm focus:outline-none"
        />{#if node.type === "file"}<span class="text-muted-foreground text-sm"
            >.doc</span
          >{/if}
      </div>
    {:else}
      <ContextMenu.Root>
        <ContextMenu.Trigger>
          <button
            onclick={handleClick}
            draggable={true}
            ondragstart={handleDragStart}
            onmouseenter={() => onHover(node.path)}
            onmouseleave={() => onHover(null)}
            data-path={node.path}
            class="focus-visible:ring-ring flex w-full items-center gap-2 rounded px-2 py-1 text-left text-sm transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-offset-1
              {isSelected ? 'bg-primary/20 text-primary' : 'hover:bg-accent'}
              {node.type === 'folder' && dropTargetPath === node.path
              ? 'bg-primary/10 ring-primary cursor-move ring-2'
              : ''}"
            style="padding-left: {depth * 12 + 8}px"
          >
            {#if node.type === "folder"}
              <ChevronRight
                class="text-muted-foreground h-3 w-3 shrink-0 transition-transform {expanded
                  ? 'rotate-90'
                  : ''}"
              />
              <Folder
                class="h-4 w-4 shrink-0 {node.isTemporary
                  ? 'text-muted-foreground/50'
                  : 'text-muted-foreground'}"
              />
            {:else}
              <span class="w-3 shrink-0"></span>
              <FileText class="text-muted-foreground h-4 w-4 shrink-0" />
            {/if}
            <span
              class="truncate {node.isTemporary
                ? 'text-muted-foreground italic'
                : ''}">{node.name}</span
            >
          </button>
        </ContextMenu.Trigger>

        <ContextMenu.Content class="w-48">
          <ContextMenu.Item onclick={() => onCreateDocument(getTargetFolder())}>
            <FilePlus class="mr-2 h-4 w-4" />
            {m.sidebar_new_document()}
          </ContextMenu.Item>

          <ContextMenu.Item onclick={() => onCreateFolder(getTargetFolder())}>
            <FolderPlus class="mr-2 h-4 w-4" />
            {m.sidebar_new_folder()}
          </ContextMenu.Item>

          <ContextMenu.Separator />

          <ContextMenu.Item onclick={() => onRename(node)}>
            <Pencil class="mr-2 h-4 w-4" />
            {m.sidebar_rename()}
          </ContextMenu.Item>

          {#if node.type === "file" && onShare}
            <ContextMenu.Item onclick={() => onShare(node)}>
              <Share2 class="mr-2 h-4 w-4" />
              {m.share_context_menu_share()}
            </ContextMenu.Item>
          {/if}

          <ContextMenu.Item
            class="text-destructive focus:text-destructive"
            onclick={() => onDelete(node)}
          >
            <Trash2 class="mr-2 h-4 w-4" />
            {node.type === "folder"
              ? m.sidebar_delete_folder()
              : m.sidebar_delete()}
          </ContextMenu.Item>
        </ContextMenu.Content>
      </ContextMenu.Root>
    {/if}
  </div>

  {#if node.type === "folder" && expanded}
    <ul>
      {#if newFolderState?.active && newFolderState.parentPath === node.path}
        <li>
          <div
            class="bg-accent/50 flex items-center gap-2 rounded px-2 py-1"
            style="padding-left: {(depth + 1) * 12 + 8}px"
          >
            <span class="w-3 shrink-0"></span>
            <Folder class="text-muted-foreground h-4 w-4 shrink-0" />
            <input
              type="text"
              data-newfolder-input
              value={newFolderState.value}
              oninput={(e) => {
                onNewFolderInput?.(e);
                onNewFolderValueChange?.((e.target as HTMLInputElement).value);
              }}
              onkeydown={onNewFolderKeydown}
              onblur={onNewFolderBlur}
              class="h-5 flex-1 bg-transparent text-sm focus:outline-none"
            />
          </div>
        </li>
      {/if}
      {#each node.children as child (child.path)}
        <FileTreeNode
          node={child}
          depth={depth + 1}
          {onFileClick}
          {onCreateDocument}
          {onCreateFolder}
          {onRename}
          {onDelete}
          {onShare}
          {onDrop}
          {onSelect}
          {onHover}
          {onDragTarget}
          {dropTargetPath}
          {newFolderState}
          {onNewFolderInput}
          {onNewFolderKeydown}
          {onNewFolderBlur}
          {onNewFolderValueChange}
          {renameState}
          {onRenameInput}
          {onRenameKeydown}
          {onRenameBlur}
          {onRenameValueChange}
        />
      {/each}
    </ul>
  {/if}
</li>
