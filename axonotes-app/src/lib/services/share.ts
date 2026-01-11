import {invoke} from "@tauri-apps/api/core";
import {listen, type UnlistenFn} from "@tauri-apps/api/event";

/**
 * Share role types
 */
export type ShareRole = "owner" | "editor" | "reader";

/**
 * Collaborator information returned by the backend
 */
export interface Collaborator {
  userId: string;
  role: ShareRole;
}

/**
 * Share code event payload (camelCase to match backend serde)
 */
export interface ShareCodeReadyPayload {
  docId: string;
  shareCode: string;
}

/**
 * Share Service
 * Wraps Tauri commands for document sharing operations
 */
export class ShareService {
  /**
   * Create a share session for a document
   * The share code will be emitted via "share-code-ready" event
   * @param docId - The document to share
   * @param role - Role to grant to joiners ("editor" or "reader")
   * @param fullHistory - If true, joiners get all historical keys
   */
  static async createShare(
    docId: string,
    role: "editor" | "reader",
    fullHistory: boolean
  ): Promise<void> {
    await invoke("create_share", {docId, role, fullHistory});
  }

  /**
   * Join an existing share using a share code
   * @param shareCode - The share code (will be auto-uppercased)
   */
  static async joinShare(shareCode: string): Promise<void> {
    await invoke("join_share", {shareCode: shareCode.toUpperCase()});
  }

  /**
   * Close an active share session
   * @param docId - The document ID
   */
  static async closeShare(docId: string): Promise<void> {
    await invoke("close_share", {docId});
  }

  /**
   * Leave a share that you've joined (as a joiner, not creator)
   * @param shareCode - The share code to leave
   */
  static async leaveShare(shareCode: string): Promise<void> {
    await invoke("leave_share", {shareCode});
  }

  /**
   * Update a user's role on a document
   * @param docId - The document ID
   * @param userId - The user's identity as hex string
   * @param role - The new role ("editor" or "reader")
   */
  static async updateUserRole(
    docId: string,
    userId: string,
    role: "editor" | "reader"
  ): Promise<void> {
    await invoke("update_user_role", {docId, userId, role});
  }

  /**
   * Transfer document ownership to another user
   * @param docId - The document ID
   * @param newOwnerId - The new owner's identity as hex string
   */
  static async transferOwnership(
    docId: string,
    newOwnerId: string
  ): Promise<void> {
    await invoke("transfer_ownership", {docId, newOwnerId});
  }

  /**
   * Remove a user from a document
   * @param docId - The document ID
   * @param userId - The user's identity as hex string
   */
  static async removeUser(docId: string, userId: string): Promise<void> {
    await invoke("remove_user", {docId, userId});
  }

  /**
   * Get all collaborators for a document
   * @param docId - The document ID
   * @returns List of collaborators with their roles
   */
  static async getCollaborators(docId: string): Promise<Collaborator[]> {
    return invoke<Collaborator[]>("get_document_collaborators", {docId});
  }

  /**
   * Listen for share-code-ready events
   * @param callback - Function to call when share code is ready
   * @returns Unlisten function
   */
  static async onShareCodeReady(
    callback: (payload: ShareCodeReadyPayload) => void
  ): Promise<UnlistenFn> {
    return listen<ShareCodeReadyPayload>("share-code-ready", (event) => {
      callback(event.payload);
    });
  }

  /**
   * Listen for share-closed events
   * @param callback - Function to call when share is closed
   * @returns Unlisten function
   */
  static async onShareClosed(
    callback: (payload: {docId: string; shareCode: string}) => void
  ): Promise<UnlistenFn> {
    return listen<{docId: string; shareCode: string}>(
      "share-closed",
      (event) => {
        callback(event.payload);
      }
    );
  }

  /**
   * Listen for collaborator-added events
   * @param callback - Function to call when collaborator is added
   * @returns Unlisten function
   */
  static async onCollaboratorAdded(
    callback: (payload: {docId: string; userId: string; role: string}) => void
  ): Promise<UnlistenFn> {
    return listen<{docId: string; userId: string; role: string}>(
      "collaborator-added",
      (event) => {
        callback(event.payload);
      }
    );
  }

  /**
   * Listen for collaborator-removed events
   * @param callback - Function to call when collaborator is removed
   * @returns Unlisten function
   */
  static async onCollaboratorRemoved(
    callback: (payload: {docId: string; userId: string}) => void
  ): Promise<UnlistenFn> {
    return listen<{docId: string; userId: string}>(
      "collaborator-removed",
      (event) => {
        callback(event.payload);
      }
    );
  }

  /**
   * Listen for collaborator-role-changed events
   * @param callback - Function to call when role changes
   * @returns Unlisten function
   */
  static async onCollaboratorRoleChanged(
    callback: (payload: {
      docId: string;
      userId: string;
      oldRole: string;
      newRole: string;
    }) => void
  ): Promise<UnlistenFn> {
    return listen<{
      docId: string;
      userId: string;
      oldRole: string;
      newRole: string;
    }>("collaborator-role-changed", (event) => {
      callback(event.payload);
    });
  }

  /**
   * Listen for ownership-transferred events
   * @param callback - Function to call when ownership transfers
   * @returns Unlisten function
   */
  static async onOwnershipTransferred(
    callback: (payload: {
      docId: string;
      oldOwnerId: string;
      newOwnerId: string;
    }) => void
  ): Promise<UnlistenFn> {
    return listen<{docId: string; oldOwnerId: string; newOwnerId: string}>(
      "ownership-transferred",
      (event) => {
        callback(event.payload);
      }
    );
  }
}
