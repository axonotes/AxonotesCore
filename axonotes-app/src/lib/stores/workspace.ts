/**
 * Workspace Store
 *
 * Manages workspace state including the list of workspaces,
 * active workspace selection, and layout persistence.
 */

import {derived, get, writable} from "svelte/store";
import {WorkspaceService, type Workspace} from "$lib/services/workspace";

// ============================================================================
// LocalStorage Keys
// ============================================================================

const ACTIVE_WORKSPACE_KEY = "axonotes:active-workspace-id";

/**
 * Load active workspace ID from localStorage.
 */
function loadActiveWorkspaceId(): string | null {
  if (typeof localStorage === "undefined") return null;
  return localStorage.getItem(ACTIVE_WORKSPACE_KEY);
}

/**
 * Save active workspace ID to localStorage.
 */
function saveActiveWorkspaceId(id: string | null): void {
  if (typeof localStorage === "undefined") return;
  if (id) {
    localStorage.setItem(ACTIVE_WORKSPACE_KEY, id);
  } else {
    localStorage.removeItem(ACTIVE_WORKSPACE_KEY);
  }
}

/**
 * Default dockview layout configuration.
 * Creates a sidebar (30%) + main panel (70%) layout.
 * Uses dockview's SerializedDockview format for full state persistence.
 */
export const DEFAULT_LAYOUT = {
  grid: {
    root: {
      type: "branch",
      data: [
        {
          type: "leaf",
          data: {
            id: "group-sidebar",
            views: ["sidebar"],
            activeView: "sidebar",
          },
          size: 300,
        },
        {
          type: "leaf",
          data: {
            id: "group-main",
            views: ["main"],
            activeView: "main",
          },
          size: 700,
        },
      ],
    },
    width: 1000,
    height: 800,
    orientation: "HORIZONTAL",
  },
  panels: {
    sidebar: {
      id: "sidebar",
      contentComponent: "sidebar",
      title: "Files",
      params: {},
    },
    main: {
      id: "main",
      contentComponent: "welcome",
      title: "Welcome",
      params: {},
    },
  },
  activeGroup: "group-main",
};

// Core state
const workspacesStore = writable<Workspace[]>([]);
const activeWorkspaceIdStore = writable<string | null>(null);
const isLoadingStore = writable(false);

// Derived state
export const workspaces = {subscribe: workspacesStore.subscribe};
export const activeWorkspaceId = {subscribe: activeWorkspaceIdStore.subscribe};
export const isLoading = {subscribe: isLoadingStore.subscribe};

export const activeWorkspace = derived(
  [workspacesStore, activeWorkspaceIdStore],
  ([$workspaces, $activeId]) =>
    $workspaces.find((w) => w.id === $activeId) ?? null
);

// Save timeout for debouncing
let saveTimeout: ReturnType<typeof setTimeout> | null = null;

// Track pending save state for beforeunload flush
let pendingSaveConfig: string | null = null;
let pendingSaveId: string | null = null;

// Auto-save active workspace ID to localStorage when it changes
activeWorkspaceIdStore.subscribe((id) => {
  saveActiveWorkspaceId(id);
});

/**
 * Generate a unique workspace ID.
 */
function generateId(): string {
  return `ws_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;
}

/**
 * Generate a default workspace name based on the count.
 */
function generateName(count: number): string {
  return `Workspace ${count + 1}`;
}

/**
 * Workspace store actions
 */
export const workspaceStore = {
  /**
   * Load all workspaces from the backend.
   * If no workspaces exist, creates a default one.
   */
  async loadWorkspaces(): Promise<void> {
    isLoadingStore.set(true);
    try {
      const list = await WorkspaceService.listWorkspaces();

      // Sort by order field in config, fallback to updated_at
      const sorted = [...list].sort((a, b) => {
        try {
          const configA = JSON.parse(a.config);
          const configB = JSON.parse(b.config);
          const orderA = configA.order ?? Infinity;
          const orderB = configB.order ?? Infinity;
          if (orderA !== orderB) return orderA - orderB;
        } catch {
          // Fallback to updated_at if config parsing fails
        }
        return b.updated_at - a.updated_at;
      });

      workspacesStore.set(sorted);

      if (sorted.length === 0) {
        // Create default workspace
        await this.createWorkspace();
      } else {
        // Try to restore the previously active workspace from localStorage
        const savedActiveId = loadActiveWorkspaceId();
        const savedWorkspace = savedActiveId
          ? sorted.find((w) => w.id === savedActiveId)
          : null;

        if (savedWorkspace) {
          // Restore to previously active workspace
          activeWorkspaceIdStore.set(savedWorkspace.id);
        } else {
          // Fallback to most recently updated workspace
          const byDate = [...sorted].sort(
            (a, b) => b.updated_at - a.updated_at
          );
          activeWorkspaceIdStore.set(byDate[0].id);
        }
      }
    } catch (error) {
      console.error("[Workspace] Failed to load workspaces:", error);
      workspacesStore.set([]);
    } finally {
      isLoadingStore.set(false);
    }
  },

  /**
   * Create a new workspace with default layout.
   *
   * @param name - Optional workspace name (stored in config)
   * @returns The created workspace
   */
  async createWorkspace(name?: string): Promise<Workspace | null> {
    try {
      const currentList = get(workspacesStore);
      const id = generateId();
      const workspaceName = name ?? generateName(currentList.length);

      const config = JSON.stringify({
        ...DEFAULT_LAYOUT,
        name: workspaceName,
        order: currentList.length, // New workspace goes at the end
      });

      const workspace = await WorkspaceService.createWorkspace(id, config);

      workspacesStore.update((list) => [...list, workspace]);
      activeWorkspaceIdStore.set(workspace.id);

      console.log(`[Workspace] Created workspace: ${workspaceName}`);
      return workspace;
    } catch (error) {
      console.error("[Workspace] Failed to create workspace:", error);
      return null;
    }
  },

  /**
   * Switch to a different workspace.
   *
   * @param id - Workspace ID to switch to
   */
  switchWorkspace(id: string): void {
    const list = get(workspacesStore);
    const workspace = list.find((w) => w.id === id);

    if (workspace) {
      activeWorkspaceIdStore.set(id);
      console.log(`[Workspace] Switched to: ${id}`);
    } else {
      console.warn(`[Workspace] Workspace not found: ${id}`);
    }
  },

  /**
   * Save the current workspace layout (debounced).
   * Call this when dockview layout changes.
   *
   * @param config - JSON-stringified dockview layout
   */
  saveLayout(config: string): void {
    const activeId = get(activeWorkspaceIdStore);
    if (!activeId) return;

    // Track pending save for flush on beforeunload
    pendingSaveConfig = config;
    pendingSaveId = activeId;

    // Minimal debounce - just coalesce rapid successive changes (e.g., during resize drag)
    // Local SQLite is fast, no need for long delays
    if (saveTimeout) {
      clearTimeout(saveTimeout);
    }

    saveTimeout = setTimeout(async () => {
      await this._commitPendingSave();
    }, 50);
  },

  /**
   * Internal: Commit the pending save to the backend.
   */
  async _commitPendingSave(): Promise<void> {
    if (!pendingSaveConfig || !pendingSaveId) return;

    const config = pendingSaveConfig;
    const id = pendingSaveId;

    // Clear pending state before async operation
    pendingSaveConfig = null;
    pendingSaveId = null;
    saveTimeout = null;

    try {
      const updated = await WorkspaceService.updateWorkspace(id, config);

      workspacesStore.update((list) =>
        list.map((w) => (w.id === id ? updated : w))
      );

      console.log(`[Workspace] Layout saved for: ${id}`);
    } catch (error) {
      console.error("[Workspace] Failed to save layout:", error);
    }
  },

  /**
   * Flush any pending layout save immediately.
   * Call this on beforeunload or visibilitychange to prevent data loss.
   */
  flushPendingSave(): void {
    if (saveTimeout) {
      clearTimeout(saveTimeout);
      saveTimeout = null;
    }

    // Trigger the save without awaiting (for synchronous beforeunload)
    if (pendingSaveConfig && pendingSaveId) {
      this._commitPendingSave();
    }
  },

  /**
   * Delete a workspace.
   * If the deleted workspace is active, switches to another one.
   *
   * @param id - Workspace ID to delete
   */
  async deleteWorkspace(id: string): Promise<boolean> {
    try {
      const list = get(workspacesStore);

      // Don't delete the last workspace
      if (list.length <= 1) {
        console.warn("[Workspace] Cannot delete the last workspace");
        return false;
      }

      const deleted = await WorkspaceService.deleteWorkspace(id);
      if (!deleted) return false;

      workspacesStore.update((list) => list.filter((w) => w.id !== id));

      // If we deleted the active workspace, switch to another
      const activeId = get(activeWorkspaceIdStore);
      if (activeId === id) {
        const remaining = get(workspacesStore);
        if (remaining.length > 0) {
          activeWorkspaceIdStore.set(remaining[0].id);
        }
      }

      console.log(`[Workspace] Deleted workspace: ${id}`);
      return true;
    } catch (error) {
      console.error("[Workspace] Failed to delete workspace:", error);
      return false;
    }
  },

  /**
   * Rename a workspace.
   *
   * @param id - Workspace ID
   * @param name - New name
   */
  async renameWorkspace(id: string, name: string): Promise<boolean> {
    try {
      const list = get(workspacesStore);
      const workspace = list.find((w) => w.id === id);
      if (!workspace) return false;

      const config = JSON.parse(workspace.config);
      config.name = name;

      const updated = await WorkspaceService.updateWorkspace(
        id,
        JSON.stringify(config)
      );

      workspacesStore.update((list) =>
        list.map((w) => (w.id === id ? updated : w))
      );

      console.log(`[Workspace] Renamed workspace ${id} to: ${name}`);
      return true;
    } catch (error) {
      console.error("[Workspace] Failed to rename workspace:", error);
      return false;
    }
  },

  /**
   * Get workspace name from config.
   *
   * @param workspace - Workspace object
   * @returns The workspace name or a fallback
   */
  getWorkspaceName(workspace: Workspace): string {
    try {
      const config = JSON.parse(workspace.config);
      return config.name ?? "Unnamed";
    } catch {
      return "Unnamed";
    }
  },

  /**
   * Switch to next workspace (for keyboard shortcut).
   */
  nextWorkspace(): void {
    const list = get(workspacesStore);
    const activeId = get(activeWorkspaceIdStore);
    if (list.length <= 1 || !activeId) return;

    const currentIndex = list.findIndex((w) => w.id === activeId);
    const nextIndex = (currentIndex + 1) % list.length;
    activeWorkspaceIdStore.set(list[nextIndex].id);
  },

  /**
   * Switch to previous workspace (for keyboard shortcut).
   */
  previousWorkspace(): void {
    const list = get(workspacesStore);
    const activeId = get(activeWorkspaceIdStore);
    if (list.length <= 1 || !activeId) return;

    const currentIndex = list.findIndex((w) => w.id === activeId);
    const prevIndex = (currentIndex - 1 + list.length) % list.length;
    activeWorkspaceIdStore.set(list[prevIndex].id);
  },

  /**
   * Switch to workspace by index (1-9 for keyboard shortcuts).
   *
   * @param index - 1-based index
   */
  switchToIndex(index: number): void {
    const list = get(workspacesStore);
    if (index < 1 || index > list.length) return;
    activeWorkspaceIdStore.set(list[index - 1].id);
  },

  /**
   * Reorder workspaces by moving one workspace to a new position.
   * Persists the new order to the database.
   *
   * @param fromIndex - Current index of the workspace
   * @param toIndex - Target index to move to
   */
  async reorderWorkspaces(fromIndex: number, toIndex: number): Promise<void> {
    if (fromIndex === toIndex) return;

    // Update local state immediately for responsive UI
    workspacesStore.update((list) => {
      const newList = [...list];
      const [moved] = newList.splice(fromIndex, 1);
      newList.splice(toIndex, 0, moved);
      return newList;
    });

    console.log(
      `[Workspace] Reordered workspace from ${fromIndex} to ${toIndex}`
    );

    // Persist new order to database
    try {
      const list = get(workspacesStore);
      const updatePromises = list.map(async (workspace, index) => {
        const config = JSON.parse(workspace.config);
        // Only update if order changed
        if (config.order !== index) {
          config.order = index;
          const updated = await WorkspaceService.updateWorkspace(
            workspace.id,
            JSON.stringify(config)
          );
          return updated;
        }
        return workspace;
      });

      const updatedWorkspaces = await Promise.all(updatePromises);
      workspacesStore.set(updatedWorkspaces);
      console.log("[Workspace] Order persisted to database");
    } catch (error) {
      console.error("[Workspace] Failed to persist order:", error);
    }
  },
};
