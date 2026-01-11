<script lang="ts">
  import {Plus, Pencil, Trash2, CornerDownLeft} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";
  import {
    workspaces,
    activeWorkspaceId,
    workspaceStore,
  } from "$lib/stores/workspace";
  import type {Workspace} from "$lib/services/workspace";
  import {cn} from "$lib/utils";

  // =============================================================================
  // STATE MACHINE
  // =============================================================================

  // --- Modal State (only one modal can be open at a time) ---
  type Modal =
    | {type: "none"}
    | {
        type: "contextMenu";
        workspaceId: string;
        position: {x: number; y: number};
      }
    | {
        type: "rename";
        workspaceId: string;
        position: {x: number; y: number};
        value: string;
      }
    | {type: "delete"; workspaceId: string}
    | {type: "create"; value: string};

  let modal = $state<Modal>({type: "none"});

  // --- Drag State ---
  // During drag, we DON'T reorder the store. We only track:
  // - originalIndex: the workspace's actual index in the store
  // - visualIndex: where it would be inserted (for other items to shift around)
  // On release, we commit the reorder to the store.
  type Drag =
    | {phase: "none"}
    | {phase: "pending"; index: number; startX: number; startTime: number}
    | {
        phase: "active";
        originalIndex: number;
        visualIndex: number;
        baseX: number;
      };

  let drag = $state<Drag>({phase: "none"});
  let mouseX = $state(0);

  // --- Hover State ---
  let containerHovered = $state(false);
  let expandAnimationDone = $state(false);
  let hoveredWorkspaceId = $state<string | null>(null);
  let hoveredElement = $state<HTMLElement | null>(null);

  // --- Animation State ---
  // Flag to disable transitions when committing reorder (prevents double animation)
  let isCommitting = $state(false);

  // --- Refs ---
  let tooltipEl = $state<HTMLDivElement | null>(null);
  let renameInputEl = $state<HTMLInputElement | null>(null);
  let createInputEl = $state<HTMLInputElement | null>(null);
  let buttonRefs = $state<HTMLButtonElement[]>([]);

  // --- Timers ---
  let expandTimer: ReturnType<typeof setTimeout> | null = null;

  // =============================================================================
  // DERIVED STATE
  // =============================================================================

  const isExpanded = $derived(
    containerHovered || modal.type !== "none" || drag.phase !== "none"
  );

  const isModalOpen = $derived(modal.type !== "none");

  const isDragging = $derived(drag.phase === "active");

  // Get a button's width (active workspace is wider than dots)
  function getButtonWidth(index: number): number {
    const btn = buttonRefs[index];
    if (btn) return btn.offsetWidth;
    // Fallback: active workspace ~32px, dots ~16px
    return $workspaces[index]?.id === $activeWorkspaceId ? 32 : 16;
  }

  // Calculate offset for the dragged item (follows mouse exactly)
  const draggedItemOffset = $derived.by(() => {
    if (drag.phase !== "active") return 0;

    const scale = 1.5;
    // Offset = how far mouse moved from the button's original center
    const offset = (mouseX - drag.baseX) / scale;

    // Clamp to bounds - calculate actual bounds based on button positions
    const activeWidth = getButtonWidth(drag.originalIndex);
    let maxLeft = 0;
    let maxRight = 0;

    // Sum widths to the left of original position
    for (let i = 0; i < drag.originalIndex; i++) {
      maxLeft -= getButtonWidth(i);
    }
    // Sum widths to the right of original position
    for (let i = drag.originalIndex + 1; i < $workspaces.length; i++) {
      maxRight += getButtonWidth(i);
    }

    return Math.max(maxLeft - 5, Math.min(maxRight + 5, offset));
  });

  // Calculate offset for non-dragged items based on visual reordering
  function getItemOffset(index: number): number {
    if (drag.phase !== "active") return 0;
    if (index === drag.originalIndex) return draggedItemOffset;

    // The active workspace width (this is what items shift by)
    const activeWidth = getButtonWidth(drag.originalIndex);
    const {originalIndex, visualIndex} = drag;

    // Item needs to shift if dragged item "passed" it
    if (originalIndex < visualIndex) {
      // Dragged right: items between original and visual shift LEFT by active's width
      if (index > originalIndex && index <= visualIndex) {
        return -activeWidth;
      }
    } else if (originalIndex > visualIndex) {
      // Dragged left: items between visual and original shift RIGHT by active's width
      if (index >= visualIndex && index < originalIndex) {
        return activeWidth;
      }
    }

    return 0;
  }

  // =============================================================================
  // EFFECTS
  // =============================================================================

  // Initialize button refs when workspaces change
  $effect(() => {
    if (buttonRefs.length !== $workspaces.length) {
      buttonRefs = new Array($workspaces.length).fill(null);
    }
  });

  // Cleanup listeners on unmount
  $effect(() => {
    return () => {
      document.removeEventListener("pointermove", handlePointerMove);
      document.removeEventListener("pointerup", handlePointerUp);
      if (expandTimer) clearTimeout(expandTimer);
    };
  });

  // =============================================================================
  // HELPERS
  // =============================================================================

  function getWorkspaceName(workspace: Workspace): string {
    return workspaceStore.getWorkspaceName(workspace);
  }

  // =============================================================================
  // TOOLTIP
  // =============================================================================

  let tooltipStyle = $state("");

  function updateTooltipPosition(targetRect: DOMRect) {
    if (!tooltipEl) {
      tooltipStyle = `left: ${targetRect.left}px; top: ${targetRect.top - 8}px; transform: translateY(-100%);`;
      return;
    }

    const tooltipRect = tooltipEl.getBoundingClientRect();
    const padding = 8;

    let x = targetRect.left + targetRect.width / 2 - tooltipRect.width / 2;
    x = Math.max(
      padding,
      Math.min(x, window.innerWidth - tooltipRect.width - padding)
    );

    const y = targetRect.top - tooltipRect.height - 8;
    tooltipStyle = `left: ${x}px; top: ${y}px;`;
  }

  // =============================================================================
  // MODAL ACTIONS
  // =============================================================================

  function closeModal() {
    modal = {type: "none"};
    // Reset hover states so nav can shrink
    containerHovered = false;
    expandAnimationDone = false;
    isOverModal = false;
  }

  function openContextMenu(workspaceId: string, x: number, y: number) {
    modal = {type: "contextMenu", workspaceId, position: {x, y}};
    hoveredWorkspaceId = null;
  }

  function startRename() {
    if (modal.type !== "contextMenu") return;
    const {workspaceId, position} = modal;
    const workspace = $workspaces.find((w) => w.id === workspaceId);
    if (!workspace) return;

    modal = {
      type: "rename",
      workspaceId,
      position,
      value: getWorkspaceName(workspace),
    };

    requestAnimationFrame(() => {
      renameInputEl?.focus();
      renameInputEl?.select();
    });
  }

  async function submitRename() {
    if (modal.type !== "rename" || !modal.value.trim()) return;
    await workspaceStore.renameWorkspace(modal.workspaceId, modal.value.trim());
    closeModal();
  }

  function startDelete() {
    if (modal.type !== "contextMenu") return;
    if ($workspaces.length <= 1) {
      closeModal();
      return;
    }
    modal = {type: "delete", workspaceId: modal.workspaceId};
  }

  async function confirmDelete() {
    if (modal.type !== "delete") return;
    await workspaceStore.deleteWorkspace(modal.workspaceId);
    closeModal();
  }

  function startCreate() {
    modal = {type: "create", value: ""};
    requestAnimationFrame(() => createInputEl?.focus());
  }

  async function submitCreate() {
    if (modal.type !== "create") return;
    await workspaceStore.createWorkspace(modal.value.trim() || undefined);
    closeModal();
  }

  function updateModalValue(value: string) {
    if (modal.type === "rename") {
      modal = {...modal, value};
    } else if (modal.type === "create") {
      modal = {...modal, value};
    }
  }

  // =============================================================================
  // HOVER HANDLERS
  // =============================================================================

  function handleContainerEnter() {
    containerHovered = true;
    if (expandTimer) clearTimeout(expandTimer);
    expandTimer = setTimeout(() => {
      expandAnimationDone = true;
      // Recalculate tooltip after animation
      if (hoveredElement) {
        const rect = hoveredElement.getBoundingClientRect();
        updateTooltipPosition(rect);
      }
    }, 200);
  }

  function handleContainerLeave() {
    containerHovered = false;
    expandAnimationDone = false;
    hoveredWorkspaceId = null;
    hoveredElement = null;
    if (expandTimer) clearTimeout(expandTimer);
  }

  function handleWorkspaceEnter(event: MouseEvent, workspaceId: string) {
    if (isDragging) return;
    hoveredWorkspaceId = workspaceId;
    hoveredElement = event.target as HTMLElement;
    const rect = hoveredElement.getBoundingClientRect();
    updateTooltipPosition(rect);
  }

  function handleWorkspaceLeave() {
    if (isDragging) return;
    hoveredWorkspaceId = null;
    hoveredElement = null;

    // Close context menu / rename if mouse leaves (but not if over the modal)
    setTimeout(() => {
      if (
        !isOverModal &&
        (modal.type === "contextMenu" || modal.type === "rename")
      ) {
        closeModal();
      }
    }, 100);
  }

  // =============================================================================
  // DRAG HANDLERS
  // =============================================================================

  const DRAG_THRESHOLD = 3;

  function handlePointerDown(event: PointerEvent, index: number) {
    if (modal.type !== "none" || event.button !== 0) return;

    const workspace = $workspaces[index];

    // Non-active workspaces: just switch on click
    if (workspace.id !== $activeWorkspaceId) {
      workspaceStore.switchWorkspace(workspace.id);
      return;
    }

    // Only allow drag if multiple workspaces
    if ($workspaces.length <= 1) return;

    event.preventDefault();
    drag = {
      phase: "pending",
      index,
      startX: event.clientX,
      startTime: Date.now(),
    };
    mouseX = event.clientX;

    (event.target as HTMLElement).setPointerCapture(event.pointerId);
    document.addEventListener("pointermove", handlePointerMove);
    document.addEventListener("pointerup", handlePointerUp);
  }

  function handlePointerMove(event: PointerEvent) {
    if (drag.phase === "none") return;

    mouseX = event.clientX;

    // Check if we've moved enough to start dragging
    if (drag.phase === "pending") {
      const delta = Math.abs(mouseX - drag.startX);
      if (delta > DRAG_THRESHOLD) {
        // Get the button's center position (this is our baseX)
        const btn = buttonRefs[drag.index];
        const rect = btn?.getBoundingClientRect();
        const baseX = rect ? rect.left + rect.width / 2 : drag.startX;

        drag = {
          phase: "active",
          originalIndex: drag.index,
          visualIndex: drag.index,
          baseX,
        };
        hoveredWorkspaceId = null;
      }
    }

    // Update visual index based on current position
    if (drag.phase === "active") {
      updateVisualIndex();
    }
  }

  function updateVisualIndex() {
    if (drag.phase !== "active") return;

    const scale = 1.5;
    const offset = (mouseX - drag.baseX) / scale;
    const {originalIndex} = drag;

    // Calculate visual index based on cumulative button widths
    let newVisualIndex = originalIndex;

    if (offset > 0) {
      // Moving right - check each position to the right
      let cumulative = 0;
      for (let i = originalIndex + 1; i < $workspaces.length; i++) {
        // Threshold is half the width of the item we're passing
        cumulative += getButtonWidth(i);
        if (offset > cumulative - getButtonWidth(i) / 2) {
          newVisualIndex = i;
        } else {
          break;
        }
      }
    } else if (offset < 0) {
      // Moving left - check each position to the left
      let cumulative = 0;
      for (let i = originalIndex - 1; i >= 0; i--) {
        cumulative -= getButtonWidth(i);
        if (offset < cumulative + getButtonWidth(i) / 2) {
          newVisualIndex = i;
        } else {
          break;
        }
      }
    }

    if (newVisualIndex !== drag.visualIndex) {
      drag = {...drag, visualIndex: newVisualIndex};
    }
  }

  function handlePointerUp() {
    document.removeEventListener("pointermove", handlePointerMove);
    document.removeEventListener("pointerup", handlePointerUp);

    // If it was a click (pending, not moved), switch workspace
    if (drag.phase === "pending") {
      const timeDelta = Date.now() - drag.startTime;
      if (timeDelta < 200) {
        workspaceStore.switchWorkspace($workspaces[drag.index].id);
      }
    }

    // Commit the reorder if position changed
    if (drag.phase === "active" && drag.originalIndex !== drag.visualIndex) {
      // Disable transitions while DOM reorders to prevent double animation
      isCommitting = true;
      workspaceStore.reorderWorkspaces(drag.originalIndex, drag.visualIndex);

      // Re-enable transitions after DOM settles
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          isCommitting = false;
        });
      });
    }

    drag = {phase: "none"};
  }

  function handleContextMenuDuringDrag(event: MouseEvent) {
    if (isDragging) {
      event.preventDefault();
      document.removeEventListener("pointermove", handlePointerMove);
      document.removeEventListener("pointerup", handlePointerUp);
      drag = {phase: "none"};
    }
  }

  // =============================================================================
  // KEYBOARD HANDLERS
  // =============================================================================

  function handleKeydown(event: KeyboardEvent, workspaceId: string) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      workspaceStore.switchWorkspace(workspaceId);
    }
  }

  function handleModalKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      if (modal.type === "rename") submitRename();
      else if (modal.type === "create") submitCreate();
    } else if (event.key === "Escape") {
      event.preventDefault();
      closeModal();
    }
  }

  // =============================================================================
  // WINDOW CLICK (close modals)
  // =============================================================================

  function handleWindowClick(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (target.closest("[data-workspace-modal]")) return;

    if (modal.type !== "none") {
      closeModal();
    }
  }

  // =============================================================================
  // CONTEXT MENU HOVER TRACKING
  // =============================================================================

  let isOverModal = $state(false);

  function handleModalEnter() {
    isOverModal = true;
  }

  function handleModalLeave() {
    isOverModal = false;
    if (modal.type === "contextMenu" || modal.type === "rename") {
      closeModal();
    }
  }
</script>

<svelte:window onclick={handleWindowClick} />

<!-- Tooltip -->
{#if hoveredWorkspaceId && expandAnimationDone && modal.type === "none" && !isDragging}
  {@const workspace = $workspaces.find((w) => w.id === hoveredWorkspaceId)}
  {#if workspace}
    <div
      bind:this={tooltipEl}
      class="bg-popover text-popover-foreground pointer-events-none fixed z-[60] rounded-md px-2 py-1 text-xs whitespace-nowrap shadow-sm"
      style={tooltipStyle}
    >
      {getWorkspaceName(workspace)}
    </div>
  {/if}
{/if}

<!-- Context Menu -->
{#if modal.type === "contextMenu"}
  <div
    class="bg-popover text-popover-foreground fixed z-[70] min-w-32 rounded-md border p-1 shadow-md"
    style="left: {modal.position.x}px; top: {modal.position.y -
      12}px; transform: translateY(-100%);"
    role="menu"
    tabindex="-1"
    data-workspace-modal
    onmouseenter={handleModalEnter}
    onmouseleave={handleModalLeave}
  >
    <button
      type="button"
      class="hover:bg-accent flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-sm transition-colors duration-150"
      onclick={startRename}
      role="menuitem"
    >
      <Pencil class="h-4 w-4" />
      {m.workspace_context_rename()}
    </button>
    <button
      type="button"
      class="text-destructive hover:bg-destructive/10 flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-sm transition-colors duration-150"
      onclick={startDelete}
      role="menuitem"
    >
      <Trash2 class="h-4 w-4" />
      {m.workspace_context_delete()}
    </button>
  </div>
{/if}

<!-- Rename Dialog -->
{#if modal.type === "rename"}
  <div
    class="bg-popover text-popover-foreground fixed z-[70] flex items-center gap-1 rounded-md border p-1 shadow-md"
    style="left: {modal.position.x}px; top: {modal.position.y -
      12}px; transform: translateY(-100%);"
    role="dialog"
    aria-label="Rename workspace"
    data-workspace-modal
    onmouseenter={handleModalEnter}
    onmouseleave={handleModalLeave}
  >
    <input
      bind:this={renameInputEl}
      type="text"
      value={modal.value}
      oninput={(e) => updateModalValue(e.currentTarget.value)}
      onkeydown={handleModalKeydown}
      class="border-input bg-background focus-visible:ring-ring h-7 w-32 rounded-sm border px-2 text-sm focus-visible:ring-1 focus-visible:outline-none"
      placeholder={m.workspace_rename_placeholder()}
    />
    <button
      type="button"
      class="bg-foreground text-background hover:bg-foreground/90 flex h-7 w-7 items-center justify-center rounded-sm transition-colors duration-150"
      onclick={submitRename}
      aria-label={m.common_button_save()}
    >
      <CornerDownLeft class="h-4 w-4" />
    </button>
  </div>
{/if}

<!-- Delete Confirmation -->
{#if modal.type === "delete"}
  {@const deleteId = modal.workspaceId}
  {@const workspace = $workspaces.find((w) => w.id === deleteId)}
  <div
    class="bg-popover text-popover-foreground fixed bottom-14 left-4 z-[70] flex items-center gap-2 rounded-md border p-1.5 shadow-md"
    data-workspace-modal
  >
    <span class="text-muted-foreground px-1 text-sm">
      {m.workspace_context_delete()}
      {#if workspace}
        <span class="text-foreground font-medium"
          >{getWorkspaceName(workspace)}</span
        >
      {/if}?
    </span>
    <button
      type="button"
      class="bg-destructive text-destructive-foreground hover:bg-destructive/90 flex h-7 items-center justify-center rounded-sm px-2 text-sm font-medium transition-colors duration-150"
      onclick={confirmDelete}
    >
      {m.workspace_context_delete()}
    </button>
  </div>
{/if}

<!-- Create Dialog -->
{#if modal.type === "create"}
  <div
    class="bg-popover text-popover-foreground fixed bottom-14 left-4 z-[70] flex items-center gap-1 rounded-md border p-1 shadow-md"
    data-workspace-modal
  >
    <input
      bind:this={createInputEl}
      type="text"
      value={modal.value}
      oninput={(e) => updateModalValue(e.currentTarget.value)}
      onkeydown={handleModalKeydown}
      class="border-input bg-background focus-visible:ring-ring h-7 w-32 rounded-sm border px-2 text-sm focus-visible:ring-1 focus-visible:outline-none"
      placeholder={m.workspace_rename_placeholder()}
    />
    <button
      type="button"
      class="bg-foreground text-background hover:bg-foreground/90 flex h-7 w-7 items-center justify-center rounded-sm transition-colors duration-150"
      onclick={submitCreate}
      aria-label={m.workspace_indicator_new()}
    >
      <CornerDownLeft class="h-4 w-4" />
    </button>
  </div>
{/if}

<!-- Main Navigation -->
<nav
  class={cn(
    "group/nav fixed bottom-4 left-4 z-50 flex origin-bottom-left items-center gap-0 transition-all duration-200",
    isExpanded || isOverModal ? "scale-150 opacity-100" : "opacity-40"
  )}
  style={isDragging ? "cursor: grabbing;" : ""}
  aria-label={m.workspace_indicator_aria_label()}
  onmouseenter={handleContainerEnter}
  onmouseleave={handleContainerLeave}
  oncontextmenu={handleContextMenuDuringDrag}
>
  {#each $workspaces as workspace, index (workspace.id)}
    {@const isActive = workspace.id === $activeWorkspaceId}
    {@const isBeingDragged =
      isDragging && drag.phase === "active" && drag.originalIndex === index}
    {@const highlightedId =
      modal.type === "rename" || modal.type === "contextMenu"
        ? modal.workspaceId
        : modal.type === "delete"
          ? modal.workspaceId
          : null}
    {@const isHighlighted = workspace.id === highlightedId}
    {@const offset = getItemOffset(index)}
    <button
      bind:this={buttonRefs[index]}
      type="button"
      class={cn(
        "focus-visible:ring-ring group relative flex h-6 items-center justify-center rounded-md px-1 focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none",
        isBeingDragged
          ? "z-10 cursor-grabbing"
          : isActive
            ? "cursor-grab"
            : "cursor-pointer",
        !isBeingDragged && !isCommitting && "transition-transform duration-300"
      )}
      style="transform: translateX({offset}px); {!isBeingDragged &&
      !isCommitting
        ? 'transition-timing-function: cubic-bezier(0.34, 1.56, 0.64, 1);'
        : ''}"
      onpointerdown={(e) => handlePointerDown(e, index)}
      oncontextmenu={(e) => {
        if (!isDragging) {
          e.preventDefault();
          openContextMenu(workspace.id, e.clientX, e.clientY);
        }
      }}
      onmouseenter={(e) => handleWorkspaceEnter(e, workspace.id)}
      onmouseleave={handleWorkspaceLeave}
      onkeydown={(e) => handleKeydown(e, workspace.id)}
      aria-label={getWorkspaceName(workspace)}
      aria-current={isActive ? "page" : undefined}
    >
      <span
        class={cn(
          "rounded-full transition-all duration-200",
          isActive
            ? cn(
                "w-6",
                isExpanded || isOverModal ? "h-1.5" : "h-2",
                isModalOpen
                  ? isHighlighted
                    ? "bg-foreground"
                    : "bg-muted-foreground/30"
                  : "bg-foreground"
              )
            : cn(
                "h-1.5 w-1.5",
                isModalOpen && isHighlighted
                  ? "bg-foreground"
                  : isModalOpen
                    ? "bg-muted-foreground/30"
                    : "bg-muted-foreground group-hover:bg-foreground"
              )
        )}
      ></span>
    </button>
  {/each}

  <!-- Add Button -->
  <button
    type="button"
    class={cn(
      "text-muted-foreground hover:text-foreground focus-visible:ring-ring flex h-5 w-5 items-center justify-center rounded-md transition-all duration-150 focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none",
      isExpanded || isOverModal ? "opacity-60" : "opacity-0"
    )}
    onclick={startCreate}
    aria-label={m.workspace_indicator_new()}
    data-workspace-modal
  >
    <Plus class="h-3 w-3" />
  </button>
</nav>
