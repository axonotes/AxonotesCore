/**
 * Document State Service
 *
 * Wraps the backend-managed document state API. The backend handles all
 * block state reconstruction, live update forwarding, sync integration,
 * lock management, and time-travel — the frontend simply sends user
 * actions and receives block updates through a single event channel.
 *
 * ## Usage
 *
 * ```typescript
 * // 1. Set up the event listener BEFORE opening the document
 * const unlisten = await DocumentStateService.onBlockUpdates(docId, (updates) => {
 *   for (const update of updates) {
 *     if (update.type === "set") {
 *       blocks.set(update.blockId, { block: update.block, lock: update.lock });
 *     } else if (update.type === "remove") {
 *       blocks.delete(update.blockId);
 *     }
 *   }
 * });
 *
 * // 2. Open the document (initial state arrives via event)
 * await DocumentStateService.openDocument(docId);
 *
 * // 3. Edit blocks
 * await DocumentStateService.updateBlock(docId, blockId, newContent);
 *
 * // 4. Close when done
 * await DocumentStateService.closeDocument(docId);
 * unlisten();
 * ```
 */

import {invoke} from "@tauri-apps/api/core";
import {listen, type UnlistenFn} from "@tauri-apps/api/event";
import type {Block} from "$lib/services/block";

// ========== Types ==========

/**
 * Lock state for a block held by another user.
 */
export type LockState = "locked" | "focused";

/**
 * Lock information for a block.
 */
export interface LockInfo {
  /** Identity hex string of the user holding the lock */
  userId: string;
  /** Display name of the user */
  username: string;
  /** Whether the lock is active (editing) or just focused */
  state: LockState;
}

/**
 * A block update event from the backend.
 *
 * - `set`: A block was added or its content/lock state changed.
 *   The frontend should upsert this block in its local state.
 * - `remove`: A block was removed (deleted or absent at current timestamp).
 *   The frontend should remove this block from its local state.
 */
export type BlockUpdate =
  | {
      type: "set";
      blockId: number;
      block: Block;
      lock: LockInfo | null;
    }
  | {
      type: "remove";
      blockId: number;
    };

// ========== Service ==========

/**
 * Document State Service
 *
 * Clean API for the backend-managed document state system.
 * All block state changes are pushed through the `block_update_{docId}` event.
 */
export class DocumentStateService {
  /**
   * Opens a document for editing.
   *
   * Reconstructs the current block state on the backend, sets up event
   * forwarding for live updates and sync completions, and pushes the
   * initial state through the `block_update_{docId}` event channel.
   *
   * **Important:** Set up the event listener via `onBlockUpdates()` BEFORE
   * calling this, or you will miss the initial state.
   *
   * If the document is already open, the previous session is closed first.
   *
   * @param docId - The document to open
   */
  static async openDocument(docId: string): Promise<void> {
    await invoke("open_document", {docId});
  }

  /**
   * Closes an open document.
   *
   * Releases any locks held, flushes pending persists, tears down event
   * forwarding, and clears the frontend mirror. No-op if the document
   * is not currently open.
   *
   * @param docId - The document to close
   */
  static async closeDocument(docId: string): Promise<void> {
    await invoke("close_document", {docId});
  }

  /**
   * Sets the document time for time-travel.
   *
   * Reconstructs the document at the given timestamp, diffs against what
   * the frontend currently has, and emits only the changes through the
   * event channel.
   *
   * - **History mode** (`timestamp < now`): Document becomes read-only.
   *   Live updates and sync events are ignored. Lock info is cleared.
   * - **Live mode** (`timestamp === DocumentStateService.LIVE_TIMESTAMP`):
   *   Editing is re-enabled. Live updates and sync events are forwarded again.
   *
   * @param docId - The document to time-travel
   * @param timestamp - Target timestamp in milliseconds since epoch.
   *   Use `DocumentStateService.LIVE_TIMESTAMP` to return to live mode.
   */
  static async setTime(docId: string, timestamp: number): Promise<void> {
    await invoke("set_time", {docId, timestamp});
  }

  /**
   * Updates a block's content.
   *
   * The backend handles everything automatically:
   * - Lock acquisition (on first edit)
   * - Lock switching (when editing a different block)
   * - Persistence (throttled and batched)
   * - Live broadcast to collaborators
   * - Inactivity timeout (60s auto-release)
   *
   * No echo event is emitted back for own edits.
   *
   * @param docId - Document containing the block
   * @param blockId - Block to update
   * @param content - New block content
   */
  static async updateBlock(
    docId: string,
    blockId: number,
    content: Block
  ): Promise<void> {
    await invoke("ds_update_block", {docId, blockId, content});
  }

  /**
   * Creates a new block in the document.
   *
   * Returns the real block ID immediately — no temp IDs needed.
   * The batch is flushed immediately so collaborators see the new
   * block without the usual 500ms throttle delay.
   *
   * @param docId - Document to add the block to
   * @param block - Block content (id field will be overwritten by backend)
   * @returns The generated block ID
   */
  static async createBlock(docId: string, block: Block): Promise<number> {
    return invoke<number>("ds_create_block", {docId, block});
  }

  /**
   * Soft-deletes a block by marking it as deleted.
   *
   * The block remains in history and can be viewed via time-travel.
   * If we hold a lock on this block, it is released.
   *
   * @param docId - Document containing the block
   * @param blockId - Block to delete
   */
  static async deleteBlock(docId: string, blockId: number): Promise<void> {
    await invoke("ds_delete_block", {docId, blockId});
  }

  /**
   * Subscribe to block update events for a document.
   *
   * All block state changes (initial state, live updates, sync completions,
   * lock changes, time-travel diffs) flow through this single event channel.
   *
   * **Important:** Call this BEFORE `openDocument()` to ensure you receive
   * the initial state.
   *
   * @param docId - The document to subscribe to
   * @param callback - Called with batched block updates
   * @returns An unlisten function to clean up the subscription
   */
  static async onBlockUpdates(
    docId: string,
    callback: (updates: BlockUpdate[]) => void
  ): Promise<UnlistenFn> {
    return listen<BlockUpdate[]>(`block_update_${docId}`, (event) => {
      callback(event.payload);
    });
  }

  /**
   * Sentinel timestamp value meaning "return to live mode" for `setTime()`.
   *
   * This is `Number.MAX_SAFE_INTEGER` (2^53 - 1), which maps to `u64::MAX`
   * on the backend after Tauri's JSON deserialization.
   */
  static readonly LIVE_TIMESTAMP = Number.MAX_SAFE_INTEGER;
}
