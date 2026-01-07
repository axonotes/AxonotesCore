/**
 * Workspace Service
 *
 * Wraps Tauri commands for workspace (dockview layout) management.
 * Workspaces store JSON-stringified dockview configurations.
 */

import {invoke} from "@tauri-apps/api/core";

/**
 * Workspace configuration stored in the database.
 */
export interface Workspace {
  /** Unique workspace identifier */
  id: string;
  /** JSON-stringified dockview layout configuration */
  config: string;
  /** Unix timestamp when created */
  created_at: number;
  /** Unix timestamp when last updated */
  updated_at: number;
}

export class WorkspaceService {
  /**
   * Create a new workspace with the given configuration.
   *
   * @param id - Unique workspace identifier
   * @param config - JSON-stringified dockview layout
   * @returns The created workspace
   */
  static async createWorkspace(id: string, config: string): Promise<Workspace> {
    return invoke<Workspace>("create_workspace", {id, config});
  }

  /**
   * Get a workspace by its ID.
   *
   * @param id - Workspace identifier
   * @returns The workspace if found, null otherwise
   */
  static async getWorkspace(id: string): Promise<Workspace | null> {
    return invoke<Workspace | null>("get_workspace", {id});
  }

  /**
   * List all workspaces, ordered by creation date (newest first).
   *
   * @returns Array of all workspaces
   */
  static async listWorkspaces(): Promise<Workspace[]> {
    return invoke<Workspace[]>("list_workspaces");
  }

  /**
   * Update a workspace's layout configuration.
   *
   * @param id - Workspace identifier
   * @param config - New JSON-stringified dockview layout
   * @returns The updated workspace
   */
  static async updateWorkspace(id: string, config: string): Promise<Workspace> {
    return invoke<Workspace>("update_workspace", {id, config});
  }

  /**
   * Delete a workspace.
   *
   * @param id - Workspace identifier
   * @returns True if workspace was deleted, false if not found
   */
  static async deleteWorkspace(id: string): Promise<boolean> {
    return invoke<boolean>("delete_workspace", {id});
  }
}
