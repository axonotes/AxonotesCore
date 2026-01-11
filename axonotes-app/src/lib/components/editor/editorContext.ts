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

// ========== Constants ==========

const EDITOR_CONTEXT_KEY = Symbol("editor-context");

export const USER_COLORS = [
  "#3B82F6", // Blue
  "#10B981", // Green
  "#F59E0B", // Orange
  "#EF4444", // Red
  "#8B5CF6", // Purple
  "#EC4899", // Pink
  "#14B8A6", // Teal
  "#F97316", // Amber
] as const;

const LIVE_UPDATE_THROTTLE = 100;
const PERSIST_DEBOUNCE = 300;

// ========== Types ==========

export type BlockUIState = "idle" | "focused" | "locked";

export interface LiveBlockInfo {
  userId: string;
  username: string;
  color: string;
  state: "focused" | "locked";
  content?: Block;
}

export interface EditorBlock {
  id: number;
  content: Block;
  liveInfo: LiveBlockInfo | null;
}

// ========== Context Interface ==========

export interface EditorContext {
  // Stores
  docId: Readable<string | null>;
  blocks: Writable<Map<number, EditorBlock>>;
  blockOrder: Writable<number[]>;
  focusedBlockId: Writable<number | null>;
  lockedBlockId: Writable<number | null>;
  loading: Writable<boolean>;
  error: Writable<string | null>;
  username: Writable<string>;

  // Derived
  orderedBlocks: Readable<EditorBlock[]>;
  hasContent: Readable<boolean>;

  // Functions
  loadDocument: (docId: string) => Promise<void>;
  cleanup: () => Promise<void>;
  focusBlock: (blockId: number) => void;
  lockBlock: (blockId: number) => Promise<boolean>;
  unlockBlock: () => Promise<void>;
  blurBlock: () => Promise<void>;
  updateBlockContent: (blockId: number, updates: Partial<Block>) => void;
  createBlockAfter: (
    afterBlockId: number | null,
    block: Block
  ) => Promise<number | null>;
  deleteBlock: (blockId: number) => Promise<void>;
  getNextBlockId: (blockId: number) => number | null;
  getPrevBlockId: (blockId: number) => number | null;
}

// ========== Context Creation ==========

export function createEditorContext(): EditorContext {
  // Stores
  const docId = writable<string | null>(null);
  const blocks = writable<Map<number, EditorBlock>>(new Map());
  const blockOrder = writable<number[]>([]);
  const focusedBlockId = writable<number | null>(null);
  const lockedBlockId = writable<number | null>(null);
  const loading = writable(false);
  const error = writable<string | null>(null);
  const username = writable<string>("User");

  // Private state
  let unlistenLiveBlockUpdated: UnlistenFn | null = null;
  let unlistenLiveBlockReleased: UnlistenFn | null = null;
  let liveUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let persistTimeout: ReturnType<typeof setTimeout> | null = null;
  let pendingPersist: {docId: string; blockId: number; block: Block} | null =
    null;
  let pendingLiveUpdateBlockId: number | null = null;
  const userColorMap = new Map<string, string>();

  // Derived stores
  const orderedBlocks = derived([blocks, blockOrder], ([$blocks, $order]) => {
    return $order
      .map((id) => $blocks.get(id))
      .filter((block): block is EditorBlock => block !== undefined);
  });

  const hasContent = derived(blockOrder, ($order) => $order.length > 0);

  // ========== Helper Functions ==========

  function getUserColor(userId: string): string {
    if (userColorMap.has(userId)) {
      return userColorMap.get(userId)!;
    }
    const usedColors = new Set(userColorMap.values());
    const availableColor =
      USER_COLORS.find((c) => !usedColors.has(c)) ??
      USER_COLORS[userColorMap.size % USER_COLORS.length];
    userColorMap.set(userId, availableColor);
    return availableColor;
  }

  function cancelPendingLiveUpdate(): void {
    if (liveUpdateTimeout) {
      clearTimeout(liveUpdateTimeout);
      liveUpdateTimeout = null;
    }
    pendingLiveUpdateBlockId = null;
  }

  async function flushPendingPersist(): Promise<void> {
    if (!pendingPersist) return;
    const {docId: pDocId, blockId, block} = pendingPersist;
    pendingPersist = null;
    if (persistTimeout) {
      clearTimeout(persistTimeout);
      persistTimeout = null;
    }
    try {
      await BlockService.updateBlock(pDocId, blockId, block);
    } catch (err) {
      console.error("[editor] Failed to persist block:", err);
    }
  }

  async function releaseCurrentLock(): Promise<void> {
    const currentDocId = get(docId);
    const lockId = get(lockedBlockId);
    if (lockId === null || !currentDocId) return;

    cancelPendingLiveUpdate();
    await flushPendingPersist();
    lockedBlockId.set(null);

    try {
      await BlockService.releaseLockBlur(currentDocId, lockId);
    } catch (err) {
      console.debug(
        "[editor] Lock release failed (may already be released):",
        err
      );
    }
  }

  function scheduleLiveUpdate(
    pDocId: string,
    blockId: number,
    block: Block
  ): void {
    if (liveUpdateTimeout && pendingLiveUpdateBlockId !== blockId) {
      clearTimeout(liveUpdateTimeout);
      liveUpdateTimeout = null;
    }
    if (liveUpdateTimeout) return;

    pendingLiveUpdateBlockId = blockId;
    liveUpdateTimeout = setTimeout(async () => {
      liveUpdateTimeout = null;
      pendingLiveUpdateBlockId = null;
      if (get(lockedBlockId) !== blockId) return;
      try {
        await BlockService.updateLiveBlock(pDocId, blockId, block);
      } catch (err) {
        console.error("[editor] Failed to update live block:", err);
      }
    }, LIVE_UPDATE_THROTTLE);
  }

  function schedulePersist(
    pDocId: string,
    blockId: number,
    block: Block
  ): void {
    pendingPersist = {docId: pDocId, blockId, block};
    if (persistTimeout) {
      clearTimeout(persistTimeout);
    }
    persistTimeout = setTimeout(async () => {
      persistTimeout = null;
      await flushPendingPersist();
    }, PERSIST_DEBOUNCE);
  }

  function insertPosition(before: string): string {
    const lastChar = before[before.length - 1];
    const isDigit = /\d/.test(lastChar);
    return before + (isDigit ? "a" : "0");
  }

  // ========== Event Listeners ==========

  async function setupEventListeners(currentDocId: string): Promise<void> {
    unlistenLiveBlockUpdated = await listen<DecryptedLiveBlock>(
      "live-block-updated",
      (event) => {
        const liveBlock = event.payload;
        if (liveBlock.doc_id !== currentDocId) return;

        const ourLockedId = get(lockedBlockId);
        if (liveBlock.block_id === ourLockedId) {
          return; // Don't update blocks we're editing
        }

        blocks.update((b) => {
          const existing = b.get(liveBlock.block_id);
          if (!existing) return b;
          const newBlocks = new Map(b);
          newBlocks.set(liveBlock.block_id, {
            ...existing,
            liveInfo: {
              userId: liveBlock.user_id,
              username: liveBlock.username,
              color: getUserColor(liveBlock.user_id),
              state: liveBlock.locked_at ? "locked" : "focused",
              content: liveBlock.locked_at ? liveBlock.content : undefined,
            },
          });
          return newBlocks;
        });
      }
    );

    unlistenLiveBlockReleased = await listen<{
      doc_id: string;
      block_id: number;
    }>("live-block-released", (event) => {
      if (event.payload.doc_id !== currentDocId) return;
      blocks.update((b) => {
        const existing = b.get(event.payload.block_id);
        if (!existing) return b;
        const newBlocks = new Map(b);
        newBlocks.set(event.payload.block_id, {...existing, liveInfo: null});
        return newBlocks;
      });
    });
  }

  // ========== Public API ==========

  async function loadDocument(newDocId: string): Promise<void> {
    console.log("[editor] Loading document:", newDocId);

    await cleanup();

    docId.set(newDocId);
    loading.set(true);
    error.set(null);
    blocks.set(new Map());
    blockOrder.set([]);
    focusedBlockId.set(null);
    lockedBlockId.set(null);

    try {
      const [documentData, liveLocks, myIdentityHex] = await Promise.all([
        BlockService.getBlocks(newDocId),
        BlockService.getDocumentLocks(newDocId),
        getCurrentIdentityHex(),
      ]);

      console.log("[editor] Blocks count:", documentData.blocks?.length ?? 0);

      // Clear ghost locks
      const myGhostLocks = liveLocks.filter(
        (lock) => lock.user_id === myIdentityHex
      );
      if (myGhostLocks.length > 0) {
        await Promise.all(
          myGhostLocks.map((lock) =>
            BlockService.releaseLockBlur(newDocId, lock.block_id).catch((e) =>
              console.warn("[editor] Failed to release ghost lock:", e)
            )
          )
        );
      }

      // Create live info lookup
      const liveInfoMap = new Map<number, LiveBlockInfo>();
      for (const liveBlock of liveLocks) {
        liveInfoMap.set(liveBlock.block_id, {
          userId: liveBlock.user_id,
          username: liveBlock.username,
          color: getUserColor(liveBlock.user_id),
          state: liveBlock.locked_at ? "locked" : "focused",
          content: liveBlock.locked_at ? liveBlock.content : undefined,
        });
      }

      // Build blocks map
      const blocksMap = new Map<number, EditorBlock>();
      for (const blockData of documentData.blocks) {
        if (blockData.block.deleted) continue;
        if (blockData.block.block_type === "metadata") continue;

        blocksMap.set(blockData.block_id, {
          id: blockData.block_id,
          content: blockData.block,
          liveInfo: liveInfoMap.get(blockData.block_id) ?? null,
        });
      }

      // Sort by group_id, then group_row
      const sortedIds = [...blocksMap.values()]
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

      blocks.set(blocksMap);
      blockOrder.set(sortedIds);

      await setupEventListeners(newDocId);
      console.log("[editor] Document loaded successfully");
    } catch (err) {
      console.error("[editor] Load error:", err);
      error.set(err instanceof Error ? err.message : String(err));
    } finally {
      loading.set(false);
    }
  }

  async function cleanup(): Promise<void> {
    await blurBlock();

    if (unlistenLiveBlockUpdated) {
      unlistenLiveBlockUpdated();
      unlistenLiveBlockUpdated = null;
    }
    if (unlistenLiveBlockReleased) {
      unlistenLiveBlockReleased();
      unlistenLiveBlockReleased = null;
    }
    if (liveUpdateTimeout) {
      clearTimeout(liveUpdateTimeout);
      liveUpdateTimeout = null;
    }
    if (persistTimeout) {
      clearTimeout(persistTimeout);
      persistTimeout = null;
    }

    docId.set(null);
    blocks.set(new Map());
    blockOrder.set([]);
    focusedBlockId.set(null);
    lockedBlockId.set(null);
    loading.set(false);
    error.set(null);
    userColorMap.clear();
  }

  function focusBlock(blockId: number): void {
    const currentFocused = get(focusedBlockId);
    if (currentFocused === blockId) return;
    focusedBlockId.set(blockId);
  }

  async function lockBlock(blockId: number): Promise<boolean> {
    const currentDocId = get(docId);
    const currentUsername = get(username);
    if (!currentDocId) {
      console.warn("[editor] lockBlock: no docId");
      return false;
    }

    const currentLock = get(lockedBlockId);
    if (currentLock === blockId) return true;

    const currentBlocks = get(blocks);
    const block = currentBlocks.get(blockId);
    if (!block) {
      console.warn("[editor] lockBlock: block not found:", blockId);
      return false;
    }

    if (block.liveInfo?.state === "locked") {
      console.warn("[editor] lockBlock: already locked by another user");
      return false;
    }

    if (currentLock !== null) {
      await releaseCurrentLock();
    }

    lockedBlockId.set(blockId);
    focusedBlockId.set(blockId);

    try {
      await BlockService.requestLock(
        currentDocId,
        blockId,
        block.content,
        currentUsername
      );
      return true;
    } catch (err) {
      console.error("[editor] lockBlock: request failed:", err);
      lockedBlockId.set(null);
      return false;
    }
  }

  async function unlockBlock(): Promise<void> {
    await releaseCurrentLock();
  }

  async function blurBlock(): Promise<void> {
    await releaseCurrentLock();
    focusedBlockId.set(null);
  }

  function updateBlockContent(blockId: number, updates: Partial<Block>): void {
    const currentDocId = get(docId);
    if (!currentDocId) return;

    blocks.update((b) => {
      const existing = b.get(blockId);
      if (!existing) return b;

      const newBlocks = new Map(b);
      const updatedContent = {...existing.content, ...updates};
      newBlocks.set(blockId, {...existing, content: updatedContent});

      scheduleLiveUpdate(currentDocId, blockId, updatedContent);
      schedulePersist(currentDocId, blockId, updatedContent);

      return newBlocks;
    });
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
          liveInfo: null,
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
      console.error("[editor] Failed to create block:", err);
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
        console.error("[editor] Failed to delete block:", err);
      }
    }

    blocks.update((b) => {
      const newBlocks = new Map(b);
      newBlocks.delete(blockId);
      return newBlocks;
    });

    blockOrder.update((o) => o.filter((id) => id !== blockId));

    if (get(focusedBlockId) === blockId) {
      focusedBlockId.set(null);
    }
    if (get(lockedBlockId) === blockId) {
      lockedBlockId.set(null);
    }
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

  const context: EditorContext = {
    docId,
    blocks,
    blockOrder,
    focusedBlockId,
    lockedBlockId,
    loading,
    error,
    username,
    orderedBlocks,
    hasContent,
    loadDocument,
    cleanup,
    focusBlock,
    lockBlock,
    unlockBlock,
    blurBlock,
    updateBlockContent,
    createBlockAfter,
    deleteBlock,
    getNextBlockId,
    getPrevBlockId,
  };

  setContext(EDITOR_CONTEXT_KEY, context);

  return context;
}

// ========== Context Getter ==========

export function getEditorContext(): EditorContext {
  const context = getContext<EditorContext>(EDITOR_CONTEXT_KEY);
  if (!context) {
    throw new Error("getEditorContext must be called within an EditorPanel");
  }
  return context;
}
