import {getContext, setContext} from "svelte";
import {
  get,
  writable,
  derived,
  type Writable,
  type Readable,
} from "svelte/store";
import {listen, type UnlistenFn} from "@tauri-apps/api/event";
import {
  BlockService,
  getCurrentIdentityHex,
  type Block,
  type DecryptedLiveBlock,
} from "$lib/services/block";
import {setCurrentUserIdentity} from "$lib/utils/userColors";

// ========== Constants ==========

const DOCUMENT_CONTEXT_KEY = Symbol("document-context");

// ========== Types ==========

export interface DocumentBlock {
  id: number;
  content: Block;
}

/**
 * Interface that block state managers must implement to receive remote events.
 * This is used by the document context to route events to the appropriate block.
 */
export interface BlockEventHandler {
  handleRemoteUpdate(liveBlock: DecryptedLiveBlock): void;
  handleRemoteRelease(): void;
}

// ========== Context Interface ==========

export interface DocumentContext {
  // Stores
  docId: Readable<string | null>;
  blocks: Writable<Map<number, DocumentBlock>>;
  blockOrder: Writable<number[]>;
  loading: Writable<boolean>;
  error: Writable<string | null>;
  username: Writable<string>;

  // Derived
  orderedBlocks: Readable<DocumentBlock[]>;
  hasContent: Readable<boolean>;

  // Document lifecycle
  loadDocument: (docId: string) => Promise<void>;
  cleanup: () => Promise<void>;

  // Block operations
  createBlockAfter: (
    afterBlockId: number | null,
    block: Block
  ) => Promise<number | null>;
  deleteBlock: (blockId: number) => Promise<void>;
  getNextBlockId: (blockId: number) => number | null;
  getPrevBlockId: (blockId: number) => number | null;
  updateBlockInStore: (blockId: number, content: Block) => void;
  getBlock: (blockId: number) => DocumentBlock | undefined;

  // Block state registry (for event routing)
  registerBlockHandler: (blockId: number, handler: BlockEventHandler) => void;
  unregisterBlockHandler: (blockId: number) => void;
}

// ========== Context Creation ==========

export function createDocumentContext(): DocumentContext {
  // Stores
  const docId = writable<string | null>(null);
  const blocks = writable<Map<number, DocumentBlock>>(new Map());
  const blockOrder = writable<number[]>([]);
  const loading = writable(false);
  const error = writable<string | null>(null);
  const username = writable<string>("User");

  // Private state
  let unlistenSyncCompleted: UnlistenFn | null = null;
  let unlistenLiveUpdated: UnlistenFn | null = null;
  let unlistenLiveReleased: UnlistenFn | null = null;

  // Block state registry - maps block IDs to their event handlers
  const blockHandlerRegistry = new Map<number, BlockEventHandler>();

  // Derived stores
  const orderedBlocks = derived([blocks, blockOrder], ([$blocks, $order]) => {
    return $order
      .map((id) => $blocks.get(id))
      .filter((block): block is DocumentBlock => block !== undefined);
  });

  const hasContent = derived(blockOrder, ($order) => $order.length > 0);

  // ========== Block Handler Registry ==========

  function registerBlockHandler(
    blockId: number,
    handler: BlockEventHandler
  ): void {
    blockHandlerRegistry.set(blockId, handler);
  }

  function unregisterBlockHandler(blockId: number): void {
    blockHandlerRegistry.delete(blockId);
  }

  // ========== Helper Functions ==========

  function insertPosition(before: string): string {
    const lastChar = before[before.length - 1];
    const isDigit = /\d/.test(lastChar);
    return before + (isDigit ? "a" : "0");
  }

  function sortBlockIds(blocksMap: Map<number, DocumentBlock>): number[] {
    return [...blocksMap.values()]
      .sort((a, b) => {
        const groupA = a.content.group_id ?? "main";
        const groupB = b.content.group_id ?? "main";
        if (groupA !== groupB) return groupA.localeCompare(groupB);
        const rowA = a.content.group_row ?? "0";
        const rowB = b.content.group_row ?? "0";
        if (rowA !== rowB) return rowA.localeCompare(rowB);
        return a.id - b.id;
      })
      .map((b) => b.id);
  }

  // ========== Event Listeners ==========

  async function setupEventListeners(currentDocId: string): Promise<void> {
    // Listen for sync-completed to refresh blocks when remote changes arrive
    unlistenSyncCompleted = await listen<{
      docId: string;
      batchCount: number;
    }>("sync-completed", async (event) => {
      if (event.payload.docId !== currentDocId) return;
      if (event.payload.batchCount === 0) return;

      console.log(
        "[document] Sync completed, refreshing blocks:",
        event.payload.batchCount,
        "batches"
      );

      try {
        const documentData = await BlockService.getBlocks(currentDocId);

        const newBlocksMap = new Map<number, DocumentBlock>();
        for (const blockData of documentData.blocks) {
          if (blockData.block.deleted) continue;
          if (blockData.block.block_type === "metadata") continue;

          newBlocksMap.set(blockData.block_id, {
            id: blockData.block_id,
            content: blockData.block,
          });
        }

        const sortedIds = sortBlockIds(newBlocksMap);

        blocks.set(newBlocksMap);
        blockOrder.set(sortedIds);

        console.log("[document] Blocks refreshed after sync");
      } catch (err) {
        console.error("[document] Failed to refresh blocks after sync:", err);
      }
    });

    // Listen for live block updates and route to the appropriate block handler
    unlistenLiveUpdated = await listen<DecryptedLiveBlock>(
      "live-block-updated",
      (event) => {
        const liveBlock = event.payload;

        // Filter by document
        if (liveBlock.doc_id !== currentDocId) return;

        // Route to the appropriate block handler
        const handler = blockHandlerRegistry.get(liveBlock.block_id);
        if (handler) {
          handler.handleRemoteUpdate(liveBlock);
        }
      }
    );

    // Listen for live block releases and route to the appropriate block handler
    unlistenLiveReleased = await listen<{doc_id: string; block_id: number}>(
      "live-block-released",
      (event) => {
        // Filter by document
        if (event.payload.doc_id !== currentDocId) return;

        // Route to the appropriate block handler
        const handler = blockHandlerRegistry.get(event.payload.block_id);
        if (handler) {
          handler.handleRemoteRelease();
        }
      }
    );

    console.log("[document] Event listeners ready");
  }

  // ========== Public API ==========

  async function loadDocument(newDocId: string): Promise<void> {
    console.log("[document] Loading document:", newDocId);

    await cleanup();

    docId.set(newDocId);
    loading.set(true);
    error.set(null);
    blocks.set(new Map());
    blockOrder.set([]);

    try {
      const [documentData, liveLocks, myIdentityHex] = await Promise.all([
        BlockService.getBlocks(newDocId),
        BlockService.getDocumentLocks(newDocId),
        getCurrentIdentityHex(),
      ]);

      // Set current user identity for color assignment
      setCurrentUserIdentity(myIdentityHex);

      console.log("[document] Blocks count:", documentData.blocks?.length ?? 0);

      // Clear ghost locks (locks from this user that shouldn't exist)
      const myGhostLocks = liveLocks.filter(
        (lock) => lock.user_id === myIdentityHex
      );
      if (myGhostLocks.length > 0) {
        await Promise.all(
          myGhostLocks.map((lock) =>
            BlockService.releaseLockBlur(newDocId, lock.block_id).catch((e) =>
              console.warn("[document] Failed to release ghost lock:", e)
            )
          )
        );
      }

      // Build blocks map
      const blocksMap = new Map<number, DocumentBlock>();
      for (const blockData of documentData.blocks) {
        if (blockData.block.deleted) continue;
        if (blockData.block.block_type === "metadata") continue;

        blocksMap.set(blockData.block_id, {
          id: blockData.block_id,
          content: blockData.block,
        });
      }

      const sortedIds = sortBlockIds(blocksMap);

      blocks.set(blocksMap);
      blockOrder.set(sortedIds);

      // Set up event listeners BEFORE blocks render
      // This ensures listeners exist when blocks register their handlers
      await setupEventListeners(newDocId);

      console.log("[document] Document loaded successfully");
    } catch (err) {
      console.error("[document] Load error:", err);
      error.set(err instanceof Error ? err.message : String(err));
    } finally {
      loading.set(false);
    }
  }

  async function cleanup(): Promise<void> {
    // Clean up all event listeners
    if (unlistenSyncCompleted) {
      unlistenSyncCompleted();
      unlistenSyncCompleted = null;
    }
    if (unlistenLiveUpdated) {
      unlistenLiveUpdated();
      unlistenLiveUpdated = null;
    }
    if (unlistenLiveReleased) {
      unlistenLiveReleased();
      unlistenLiveReleased = null;
    }

    // Clear the handler registry
    blockHandlerRegistry.clear();

    docId.set(null);
    blocks.set(new Map());
    blockOrder.set([]);
    loading.set(false);
    error.set(null);
  }

  async function createBlockAfter(
    afterBlockId: number | null,
    block: Block
  ): Promise<number | null> {
    const currentDocId = get(docId);
    if (!currentDocId) return null;

    try {
      const order = get(blockOrder);
      const currentBlocks = get(blocks);

      let newGroupRow: string;
      if (order.length === 0 || afterBlockId === null) {
        if (order.length === 0) {
          newGroupRow = "0";
        } else {
          const firstId = order[0];
          const firstBlock = currentBlocks.get(firstId);
          const firstRow = firstBlock?.content.group_row ?? "1";
          newGroupRow = firstRow > "0" ? "0" : "00";
        }
      } else {
        const afterIndex = order.indexOf(afterBlockId);
        const afterBlock = currentBlocks.get(afterBlockId);
        const afterRow = afterBlock?.content.group_row ?? "0";

        if (afterIndex === order.length - 1) {
          const match = afterRow.match(/^(\d+)/);
          const num = match ? parseInt(match[1], 10) : 0;
          newGroupRow = String(num + 1);
        } else {
          newGroupRow = insertPosition(afterRow);
        }
      }

      const newBlock: Block = {
        ...block,
        id: 0,
        group_row: newGroupRow,
        timestamp: 0,
      };

      const newId = await BlockService.createBlock(currentDocId, newBlock);
      const createdBlock: Block = {
        ...newBlock,
        id: newId,
        timestamp: Date.now(),
      };

      blocks.update((b) => {
        const newBlocks = new Map(b);
        newBlocks.set(newId, {
          id: newId,
          content: createdBlock,
        });
        return newBlocks;
      });

      blockOrder.update((o) => {
        const newOrder = [...o];
        if (afterBlockId === null) {
          newOrder.unshift(newId);
        } else {
          const idx = newOrder.indexOf(afterBlockId);
          newOrder.splice(idx + 1, 0, newId);
        }
        return newOrder;
      });

      return newId;
    } catch (err) {
      console.error("[document] Failed to create block:", err);
      return null;
    }
  }

  async function deleteBlock(blockId: number): Promise<void> {
    const currentDocId = get(docId);
    if (!currentDocId) return;

    try {
      await BlockService.deleteBlock(currentDocId, blockId);
    } catch (err) {
      const errorMsg = String(err);
      if (!errorMsg.includes("doesn't exist")) {
        console.error("[document] Failed to delete block:", err);
      }
    }

    blocks.update((b) => {
      const newBlocks = new Map(b);
      newBlocks.delete(blockId);
      return newBlocks;
    });

    blockOrder.update((o) => o.filter((id) => id !== blockId));
  }

  function getNextBlockId(blockId: number): number | null {
    const order = get(blockOrder);
    const idx = order.indexOf(blockId);
    if (idx === -1 || idx === order.length - 1) return null;
    return order[idx + 1];
  }

  function getPrevBlockId(blockId: number): number | null {
    const order = get(blockOrder);
    const idx = order.indexOf(blockId);
    if (idx <= 0) return null;
    return order[idx - 1];
  }

  function updateBlockInStore(blockId: number, content: Block): void {
    blocks.update((b) => {
      const existing = b.get(blockId);
      if (!existing) return b;

      const newBlocks = new Map(b);
      newBlocks.set(blockId, {...existing, content});
      return newBlocks;
    });
  }

  function getBlock(blockId: number): DocumentBlock | undefined {
    return get(blocks).get(blockId);
  }

  const context: DocumentContext = {
    docId,
    blocks,
    blockOrder,
    loading,
    error,
    username,
    orderedBlocks,
    hasContent,
    loadDocument,
    cleanup,
    createBlockAfter,
    deleteBlock,
    getNextBlockId,
    getPrevBlockId,
    updateBlockInStore,
    getBlock,
    registerBlockHandler,
    unregisterBlockHandler,
  };

  setContext(DOCUMENT_CONTEXT_KEY, context);

  return context;
}

// ========== Context Getter ==========

export function getDocumentContext(): DocumentContext {
  const context = getContext<DocumentContext>(DOCUMENT_CONTEXT_KEY);
  if (!context) {
    throw new Error("getDocumentContext must be called within an EditorPanel");
  }
  return context;
}
