/**
 * Typst Editor Context
 *
 * Manages the full lifecycle of a collaborative Typst text editor:
 * - Block state (loading, CRUD, ordering via fractional indexing)
 * - Line map (bidirectional CM6 line <-> block ID mapping)
 * - Locking (per-block locks for real-time collaboration)
 * - Live updates (throttled broadcast of edits to collaborators)
 * - Sync events (applying remote changes to the CM6 view)
 *
 * The context holds a reference to the CM6 EditorView (set via setView)
 * so it can dispatch remote changes directly into the editor.
 */

import {getContext, setContext} from "svelte";
import {Annotation, StateEffect} from "@codemirror/state";
import type {EditorView, ViewUpdate} from "@codemirror/view";
import {listen, type UnlistenFn} from "@tauri-apps/api/event";
import {
  BlockService,
  getCurrentIdentityHex,
  createTypstBlock,
  type Block,
  type DecryptedLiveBlock,
} from "$lib/services/block";
import {getUserColor, setCurrentUserIdentity} from "$lib/utils/userColors";
import {TypstLineMap} from "./typstLineMap";
import {extractChangeRegions, type ChangeRegion} from "./typstChangeProcessor";

// ========== Constants ==========

const TYPST_CONTEXT_KEY = Symbol("typst-editor-context");

const LIVE_UPDATE_THROTTLE = 100;
const PERSIST_DEBOUNCE = 300;
const RELEASE_WINDOW_MS = 50;

/**
 * Grace period (ms) after the last live block update during which we trust the
 * local content over the persisted server data.  Sync updates will skip blocks
 * that were live-updated within this window, preventing stale persisted data
 * from overwriting the fresher live content.
 *
 * 60 seconds is conservative — the persist debounce is 300 ms and the final
 * persist fires synchronously on lock release, so the server usually catches up
 * within a second.  The long grace period just adds safety margin.
 */
const LIVE_BLOCK_GRACE_MS = 60_000;

/** CM6 annotation to mark transactions dispatched by remote sync. */
export const isRemoteChange = Annotation.define<boolean>();

// ========== CM6 Effects for Lock Decorations ==========

/** Per-line lock/focus info pushed into CM6 for decoration rendering. */
export interface LineLockState {
  lineIndex: number; // 0-based
  color: string;
  username: string;
  kind: "other-locked" | "other-focused" | "own-locked" | "own-focused";
}

/** Effect to update the set of locked/focused lines in the CM6 state. */
export const setLineLocks = StateEffect.define<LineLockState[]>();

/** Effect to flash a set of lines (rejected edit feedback). */
export const flashLines = StateEffect.define<number[]>(); // 0-based line indices

// ========== Types ==========

export interface LiveBlockInfo {
  userId: string;
  username: string;
  color: string;
  state: "focused" | "locked";
  content?: Block;
}

/** Internal block record. */
interface ManagedBlock {
  id: number;
  block: Block;
}

export interface TypstEditorContext {
  loadDocument: (docId: string) => Promise<string>;
  cleanup: () => Promise<void>;
  setView: (view: EditorView) => void;
  handleDocChanged: (update: ViewUpdate) => void;
  handleCursorLine: (lineNumber: number) => void;
  handleBlur: () => Promise<void>;
  isLineLockedByOther: (lineIndex: number) => boolean;
  getLineLockInfo: (lineIndex: number) => LiveBlockInfo | null;
  flashLockedLines: (lineIndices: number[]) => void;
}

// ========== Fractional Indexing ==========

/**
 * Generate a group_row that sorts lexicographically between `before` and
 * whatever comes next. Appends 'a' after a digit, '0' after a letter.
 */
function insertPosition(before: string): string {
  const lastChar = before[before.length - 1];
  const isDigit = /\d/.test(lastChar);
  return before + (isDigit ? "a" : "0");
}

/**
 * Compute group_row values for `count` new blocks inserted after `afterRow`.
 * If `atEnd` is true the blocks are appended (use integer increment for the
 * first value, then insertPosition for subsequent ones).
 */
function computeGroupRows(
  afterRow: string | null,
  count: number,
  atEnd: boolean
): string[] {
  const rows: string[] = [];
  let prevRow = afterRow ?? "0";

  for (let i = 0; i < count; i++) {
    if (atEnd && i === 0 && afterRow !== null) {
      // First block at end of document: increment leading integer
      const match = prevRow.match(/^(\d+)/);
      const num = match ? parseInt(match[1], 10) : 0;
      prevRow = String(num + 1);
    } else if (afterRow === null && i === 0) {
      // Inserting before everything else in the doc
      prevRow = "0";
    } else {
      prevRow = insertPosition(prevRow);
    }
    rows.push(prevRow);
  }
  return rows;
}

// ========== Context Creation ==========

export function createTypstEditorContext(): TypstEditorContext {
  // ===== Temp ID generation (per-context, not shared across instances) =====
  let tempIdCounter = 0;
  function nextTempId(): number {
    return -++tempIdCounter;
  }

  // ===== Internal state =====
  let currentDocId: string | null = null;
  let editorView: EditorView | null = null;
  let myIdentityHex: string | null = null;

  const lineMap = new TypstLineMap();
  const blockData = new Map<number, ManagedBlock>();
  const liveInfo = new Map<number, LiveBlockInfo>();

  /**
   * Tracks when each block last received a live block update (timestamp).
   * While a block is within the grace period, applySyncUpdate will use the
   * local blockData content instead of the (potentially stale) server data.
   */
  const liveBlockTouched = new Map<number, number>();

  // Locking
  let lockedBlockId: number | null = null;
  let lockGeneration: number = 0; // Incremented on each tryLockLine call

  // Throttle / debounce
  let liveUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let liveUpdateBlockId: number | null = null;
  let persistTimeout: ReturnType<typeof setTimeout> | null = null;
  let pendingPersist: {docId: string; blockId: number; block: Block} | null =
    null;

  // Tauri event listeners
  let unlistenLiveUpdated: UnlistenFn | null = null;
  let unlistenLiveReleased: UnlistenFn | null = null;
  let unlistenSyncCompleted: UnlistenFn | null = null;
  const pendingReleases = new Map<number, ReturnType<typeof setTimeout>>();

  // Pending async block creations (tempId -> Promise<realId>)
  const pendingCreations = new Map<number, Promise<number>>();
  // Cancelled temp IDs (creation should be a no-op on resolve)
  const cancelledCreations = new Set<number>();

  // Current focused line (0-based), tracked for focus indicator
  let focusedLineIndex: number | null = null;

  // ===== CM6 helpers =====

  /**
   * Push the current lock/focus state into CM6 via a StateEffect.
   * Called whenever liveInfo, lockedBlockId, or focusedLineIndex changes.
   */
  function pushLockDecorations(): void {
    if (!editorView) return;

    const states: LineLockState[] = [];

    // Other users' locks/focuses from liveInfo
    for (const [blockId, info] of liveInfo) {
      const lineIdx = lineMap.getLineIndex(blockId);
      if (lineIdx === undefined) continue;
      states.push({
        lineIndex: lineIdx,
        color: info.color,
        username: info.username,
        kind: info.state === "locked" ? "other-locked" : "other-focused",
      });
    }

    // Own locked line
    if (lockedBlockId !== null) {
      const lineIdx = lineMap.getLineIndex(lockedBlockId);
      if (lineIdx !== undefined) {
        states.push({
          lineIndex: lineIdx,
          color: "var(--primary)",
          username: "",
          kind: "own-locked",
        });
      }
    }

    // Own focused line (only if not already locked)
    if (
      focusedLineIndex !== null &&
      (lockedBlockId === null ||
        lineMap.getLineIndex(lockedBlockId) !== focusedLineIndex)
    ) {
      states.push({
        lineIndex: focusedLineIndex,
        color: "var(--primary)",
        username: "",
        kind: "own-focused",
      });
    }

    editorView.dispatch({
      effects: setLineLocks.of(states),
    });
  }

  /**
   * Dispatch a remote text change for a single line into CM6.
   * Automatically annotated with isRemoteChange so our updateListener skips it.
   */
  function remoteSetLineText(lineNum: number, newText: string): void {
    if (!editorView) return;
    const doc = editorView.state.doc;
    if (lineNum < 1 || lineNum > doc.lines) return;
    const line = doc.line(lineNum);
    if (line.text === newText) return;
    editorView.dispatch({
      changes: {from: line.from, to: line.to, insert: newText},
      annotations: isRemoteChange.of(true),
    });
  }

  // ===== Throttle / debounce helpers =====

  function cancelPendingLiveUpdate(): void {
    if (liveUpdateTimeout) {
      clearTimeout(liveUpdateTimeout);
      liveUpdateTimeout = null;
    }
    liveUpdateBlockId = null;
  }

  async function flushPendingPersist(): Promise<void> {
    if (!pendingPersist) return;
    const {docId, blockId, block} = pendingPersist;
    pendingPersist = null;
    if (persistTimeout) {
      clearTimeout(persistTimeout);
      persistTimeout = null;
    }
    // Don't persist temp IDs — the creation itself persists the data
    if (blockId < 0) return;
    try {
      await BlockService.updateBlock(docId, blockId, block);
    } catch (err) {
      console.error("[typst] Failed to persist block:", err);
    }
  }

  function scheduleLiveUpdate(
    docId: string,
    blockId: number,
    _block: Block
  ): void {
    // Skip temp IDs — block doesn't exist on backend yet
    if (blockId < 0) return;

    if (liveUpdateTimeout && liveUpdateBlockId !== blockId) {
      clearTimeout(liveUpdateTimeout);
      liveUpdateTimeout = null;
    }
    if (liveUpdateTimeout) return; // Already scheduled for this block

    liveUpdateBlockId = blockId;
    liveUpdateTimeout = setTimeout(async () => {
      liveUpdateTimeout = null;
      liveUpdateBlockId = null;
      if (lockedBlockId !== blockId) return;
      // Read the LATEST content when the timeout fires, not the stale
      // content from when scheduleLiveUpdate was first called.
      const managed = blockData.get(blockId);
      if (!managed) return;
      try {
        await BlockService.updateLiveBlock(docId, blockId, managed.block);
      } catch (err) {
        console.error("[typst] Failed to update live block:", err);
      }
    }, LIVE_UPDATE_THROTTLE);
  }

  function schedulePersist(docId: string, blockId: number, block: Block): void {
    // Skip temp IDs
    if (blockId < 0) return;

    pendingPersist = {docId, blockId, block};
    if (persistTimeout) clearTimeout(persistTimeout);
    persistTimeout = setTimeout(async () => {
      persistTimeout = null;
      await flushPendingPersist();
    }, PERSIST_DEBOUNCE);
  }

  // ===== Lock management =====

  async function releaseCurrentLock(): Promise<void> {
    if (lockedBlockId === null || !currentDocId) return;
    const lockId = lockedBlockId;

    cancelPendingLiveUpdate();
    await flushPendingPersist();
    lockedBlockId = null;
    pushLockDecorations();

    // Don't try to release temp IDs (block doesn't exist on backend)
    if (lockId < 0) return;

    try {
      await BlockService.releaseLockBlur(currentDocId, lockId);
    } catch (err) {
      console.debug(
        "[typst] Lock release failed (may already be released):",
        err
      );
    }
  }

  /**
   * Try to acquire a lock on the block at the given 0-based line index.
   * Returns true if the lock was acquired (or we already held it).
   *
   * Uses a generation counter to handle rapid consecutive calls: after each
   * await, if a newer tryLockLine call has started, this one aborts.
   */
  async function tryLockLine(lineIndex: number): Promise<boolean> {
    if (!currentDocId) return false;

    const blockId = lineMap.getBlockId(lineIndex);
    if (blockId === undefined || blockId < 0) return false; // No block or pending creation

    if (lockedBlockId === blockId) return true; // Already locked

    // Check if locked by another user (sync check)
    const live = liveInfo.get(blockId);
    if (live?.state === "locked") {
      return false;
    }

    // Increment generation: any older in-flight tryLockLine calls are now stale
    const myGen = ++lockGeneration;

    // Release any existing lock first
    if (lockedBlockId !== null) {
      await releaseCurrentLock();
    }

    // Check if superseded by a newer call while we awaited
    if (myGen !== lockGeneration) return false;

    lockedBlockId = blockId;
    pushLockDecorations();

    const managed = blockData.get(blockId);
    if (!managed) {
      lockedBlockId = null;
      pushLockDecorations();
      return false;
    }

    try {
      await BlockService.requestLock(
        currentDocId,
        blockId,
        managed.block,
        "User" // TODO: get actual username from profile
      );

      // Check again after the async lock request
      if (myGen !== lockGeneration) {
        // A newer lock request superseded us — release this one
        BlockService.releaseLockBlur(currentDocId!, blockId).catch(() => {});
        return false;
      }

      return true;
    } catch (err) {
      console.error("[typst] Lock request failed:", err);
      // Only clear if we're still the active generation
      if (myGen === lockGeneration) {
        lockedBlockId = null;
        pushLockDecorations();
      }
      return false;
    }
  }

  // ===== Block CRUD =====

  /**
   * Update an existing block's text. Schedules live broadcast + persist.
   */
  function updateBlockText(blockId: number, newText: string): void {
    if (!currentDocId) return;
    const managed = blockData.get(blockId);
    if (!managed) return;

    // Skip if text hasn't actually changed
    if (managed.block.text === newText) return;

    const updatedBlock: Block = {...managed.block, text: newText};
    blockData.set(blockId, {id: blockId, block: updatedBlock});

    scheduleLiveUpdate(currentDocId, blockId, updatedBlock);
    schedulePersist(currentDocId, blockId, updatedBlock);
  }

  /**
   * Create a new block asynchronously. Returns a temp ID immediately.
   *
   * IMPORTANT: Does NOT modify the lineMap. The caller is responsible for
   * inserting the returned temp ID into the lineMap at the correct position
   * (to allow atomic batch splicing in applyChangeRegion).
   *
   * The async creation fires in the background. When it resolves, the temp ID
   * is swapped for the real ID in both lineMap and blockData.
   */
  function createBlockAsync(text: string, groupRow: string): number {
    const tempId = nextTempId();
    if (!currentDocId) return tempId;

    const docId = currentDocId;

    // Store a placeholder block record
    const placeholderBlock: Block = {
      id: 0,
      timestamp: Date.now(),
      block_type: "typst",
      block_type_version: 1,
      author: [],
      group_id: "main",
      group_row: groupRow,
      text,
    };
    blockData.set(tempId, {id: tempId, block: placeholderBlock});

    // Fire async creation
    const promise = (async () => {
      const block = await createTypstBlock(0, text, "main", groupRow);
      const realId = await BlockService.createBlock(docId, block);

      // Check if creation was cancelled (line was deleted before completion)
      if (cancelledCreations.has(tempId)) {
        cancelledCreations.delete(tempId);
        pendingCreations.delete(tempId);
        // Clean up: delete the block we just created on the backend
        try {
          await BlockService.deleteBlock(docId, realId);
        } catch {
          // Best effort cleanup
        }
        return realId;
      }

      // Swap temp -> real in lineMap
      const lineIdx = lineMap.getLineIndex(tempId);
      if (lineIdx !== undefined) {
        lineMap.setBlockId(lineIdx, realId);
      }

      // Swap in blockData
      const currentManaged = blockData.get(tempId);
      if (currentManaged) {
        const realBlock: Block = {
          ...currentManaged.block,
          id: realId,
          author: block.author,
        };
        blockData.set(realId, {id: realId, block: realBlock});

        // If text changed while creation was in flight, persist the latest
        if (currentManaged.block.text !== text) {
          try {
            await BlockService.updateBlock(docId, realId, realBlock);
          } catch (err) {
            console.error(
              "[typst] Failed to update block after creation:",
              err
            );
          }
        }
      }
      blockData.delete(tempId);
      pendingCreations.delete(tempId);

      return realId;
    })();

    pendingCreations.set(tempId, promise);
    return tempId;
  }

  /**
   * Delete a block. If the block is still pending creation (temp ID),
   * marks it for cancellation instead.
   */
  function deleteBlockById(blockId: number): void {
    if (!currentDocId) return;

    // If temp ID, just cancel the pending creation
    if (blockId < 0) {
      cancelledCreations.add(blockId);
      blockData.delete(blockId);
      return;
    }

    blockData.delete(blockId);
    liveInfo.delete(blockId);

    if (lockedBlockId === blockId) {
      lockedBlockId = null;
    }

    const docId = currentDocId;
    BlockService.deleteBlock(docId, blockId).catch((err) => {
      const msg = String(err);
      if (!msg.includes("doesn't exist")) {
        console.error("[typst] Failed to delete block:", err);
      }
    });
  }

  // ===== Change processing =====

  /**
   * Process a CM6 ViewUpdate: detect which lines changed, were added, or removed,
   * then update the block model accordingly.
   */
  function handleDocChanged(update: ViewUpdate): void {
    if (!currentDocId) return;

    const oldDoc = update.startState.doc;
    const newDoc = update.state.doc;

    // Extract change regions (returned in reverse document order)
    const regions = extractChangeRegions(oldDoc, newDoc, update.changes);

    for (const region of regions) {
      applyChangeRegion(region);
    }

    // After processing changes, try to lock the cursor's line.
    // Only acquire a lock if we don't hold ANY lock yet. If we already have
    // a lock on a different line, keep it — the user is still editing and
    // the lock will naturally move on the next change to a different line.
    if (lockedBlockId === null) {
      const mainSel = update.state.selection.main;
      const cursorLine = update.state.doc.lineAt(mainSel.head).number;
      const cursorLineIdx = cursorLine - 1;
      const cursorBlockId = lineMap.getBlockId(cursorLineIdx);

      if (cursorBlockId !== undefined && cursorBlockId > 0) {
        // Fire and forget — lock acquisition is best-effort
        tryLockLine(cursorLineIdx);
      }
    }
  }

  /**
   * Apply a single change region to the block model and lineMap.
   *
   * This performs ONE atomic splice on the lineMap for the entire region,
   * avoiding index shifting issues from individual block creates/deletes.
   */
  function applyChangeRegion(region: ChangeRegion): void {
    const {oldFirstLineIdx, oldLineCount, newLineCount, newTexts} = region;

    // Get old block IDs in the affected range
    const oldBlockIds = lineMap.getBlockIds(oldFirstLineIdx, oldLineCount);

    const commonCount = Math.min(oldLineCount, newLineCount);
    const newBlockIds: number[] = [];

    // --- Update existing blocks (pair old lines with new lines) ---
    for (let i = 0; i < commonCount; i++) {
      const blockId = oldBlockIds[i];
      if (blockId !== undefined) {
        updateBlockText(blockId, newTexts[i]);
        newBlockIds.push(blockId);
      } else {
        // Line had no block (e.g. initial empty doc) — create one.
        // Compute group_row relative to whatever we've built so far.
        const afterBlockId =
          newBlockIds.length > 0
            ? newBlockIds[newBlockIds.length - 1]
            : oldFirstLineIdx > 0
              ? lineMap.getBlockId(oldFirstLineIdx - 1)
              : undefined;
        const afterRow =
          afterBlockId !== undefined
            ? (blockData.get(afterBlockId)?.block.group_row ?? null)
            : null;
        const isAtEnd = oldFirstLineIdx + i >= lineMap.length;
        const [groupRow] = computeGroupRows(afterRow, 1, isAtEnd);
        const tempId = createBlockAsync(newTexts[i], groupRow);
        newBlockIds.push(tempId);
      }
    }

    // --- Create blocks for extra new lines ---
    if (newLineCount > oldLineCount) {
      const extraCount = newLineCount - commonCount;

      // group_row anchor: the last block ID we've collected so far
      const lastBlockId =
        newBlockIds.length > 0
          ? newBlockIds[newBlockIds.length - 1]
          : oldFirstLineIdx > 0
            ? lineMap.getBlockId(oldFirstLineIdx - 1)
            : undefined;

      const lastRow =
        lastBlockId !== undefined
          ? (blockData.get(lastBlockId)?.block.group_row ?? null)
          : null;

      const isAtEnd = oldFirstLineIdx + oldLineCount >= lineMap.length;
      const groupRows = computeGroupRows(lastRow, extraCount, isAtEnd);

      for (let i = 0; i < extraCount; i++) {
        const tempId = createBlockAsync(
          newTexts[commonCount + i],
          groupRows[i]
        );
        newBlockIds.push(tempId);
      }
    }

    // --- Delete blocks for extra old lines ---
    if (oldLineCount > newLineCount) {
      for (let i = commonCount; i < oldLineCount; i++) {
        const blockId = oldBlockIds[i];
        if (blockId !== undefined) {
          deleteBlockById(blockId);
        }
      }
    }

    // --- Atomic lineMap splice ---
    // Replace the old range with the new block IDs in one operation.
    lineMap.splice(oldFirstLineIdx, oldLineCount, ...newBlockIds);
  }

  // ===== Event listeners =====

  async function setupEventListeners(docId: string): Promise<void> {
    unlistenLiveUpdated = await listen<DecryptedLiveBlock>(
      "live-block-updated",
      (event) => {
        const lb = event.payload;
        if (lb.doc_id !== docId) return;

        // Ignore our own updates (both by locked block ID and by identity).
        // The identity check handles the case where we just released our lock
        // but the server echoes back our last live update.
        if (lb.block_id === lockedBlockId) return;
        if (lb.user_id === myIdentityHex) return;

        // Cancel any pending release for this block (sliding window)
        const pending = pendingReleases.get(lb.block_id);
        if (pending) {
          clearTimeout(pending);
          pendingReleases.delete(lb.block_id);
        }

        // Update live info
        liveInfo.set(lb.block_id, {
          userId: lb.user_id,
          username: lb.username,
          color: getUserColor(lb.user_id),
          state: lb.locked_at ? "locked" : "focused",
          content: lb.locked_at ? lb.content : undefined,
        });

        // Apply live content to CM6 if the block is locked by another user
        if (lb.locked_at && lb.content?.text !== undefined) {
          const lineIdx = lineMap.getLineIndex(lb.block_id);
          if (lineIdx !== undefined) {
            remoteSetLineText(lineIdx + 1, lb.content.text);
          }

          // Keep blockData in sync with the live content so that
          // applySyncUpdate can use the local value instead of the
          // (potentially stale) persisted server data.
          const managed = blockData.get(lb.block_id);
          if (managed) {
            blockData.set(lb.block_id, {
              id: lb.block_id,
              block: {...managed.block, text: lb.content.text},
            });
          }

          // Mark this block as recently live-updated.  applySyncUpdate
          // will defer to the local content while the grace period is active.
          liveBlockTouched.set(lb.block_id, Date.now());
        }

        // Update decorations to reflect the new lock state
        pushLockDecorations();
      }
    );

    unlistenLiveReleased = await listen<{doc_id: string; block_id: number}>(
      "live-block-released",
      (event) => {
        if (event.payload.doc_id !== docId) return;
        const blockId = event.payload.block_id;

        // Cancel any existing pending release
        const existing = pendingReleases.get(blockId);
        if (existing) clearTimeout(existing);

        // Sliding window: wait briefly before clearing liveInfo
        // because SpacetimeDB emits DELETE+INSERT for updates
        const timeout = setTimeout(() => {
          pendingReleases.delete(blockId);
          liveInfo.delete(blockId);
          pushLockDecorations();
        }, RELEASE_WINDOW_MS);

        pendingReleases.set(blockId, timeout);
      }
    );

    unlistenSyncCompleted = await listen<{docId: string; batchCount: number}>(
      "sync-completed",
      async (event) => {
        if (event.payload.docId !== docId) return;
        if (event.payload.batchCount === 0) return;
        await applySyncUpdate(docId);
      }
    );
  }

  /**
   * Re-fetch all blocks from the backend and apply changes to CM6.
   * Preserves the line the user is currently editing (locked block).
   *
   * Strategy: build the desired final block list from fresh server data,
   * reconstruct the desired CM6 text, then replace the entire document
   * in a single transaction. Update lineMap atomically afterward.
   */
  async function applySyncUpdate(docId: string): Promise<void> {
    try {
      const documentData = await BlockService.getBlocks(docId);

      // Build new ordered block list (same sorting as existing editor)
      const freshBlocks: {id: number; block: Block}[] = [];
      for (const bd of documentData.blocks) {
        if (bd.block.deleted) continue;
        if (bd.block.block_type === "metadata") continue;
        freshBlocks.push({id: bd.block_id, block: bd.block});
      }

      freshBlocks.sort((a, b) => {
        const gA = a.block.group_id ?? "main";
        const gB = b.block.group_id ?? "main";
        if (gA !== gB) return gA.localeCompare(gB);
        const rA = a.block.group_row ?? "0";
        const rB = b.block.group_row ?? "0";
        if (rA !== rB) return rA.localeCompare(rB);
        return a.id - b.id;
      });

      const freshMap = new Map(freshBlocks.map((b) => [b.id, b]));
      const freshIds = freshBlocks.map((b) => b.id);
      const freshIdSet = new Set(freshIds);
      const currentAllIds = lineMap.getAllBlockIds();
      const currentRealIdSet = new Set(currentAllIds.filter((id) => id > 0));

      // Build the desired final block ID list:
      // Start from the fresh server order, then weave in any temp IDs
      // (pending creations) at their relative positions.
      const tempPositions: {tempId: number; afterRealId: number | null}[] = [];
      for (let i = 0; i < currentAllIds.length; i++) {
        const id = currentAllIds[i];
        if (id < 0) {
          // Find the nearest real ID before this temp in the current lineMap
          let afterRealId: number | null = null;
          for (let j = i - 1; j >= 0; j--) {
            if (currentAllIds[j] > 0) {
              afterRealId = currentAllIds[j];
              break;
            }
          }
          tempPositions.push({tempId: id, afterRealId});
        }
      }

      const newBlockIds: number[] = [];

      // Temps that were before all real IDs (afterRealId === null)
      const prefixTemps = tempPositions
        .filter((tp) => tp.afterRealId === null)
        .map((tp) => tp.tempId);
      newBlockIds.push(...prefixTemps);

      // Build from fresh order, inserting temps after their anchor
      for (const id of freshIds) {
        newBlockIds.push(id);
        for (const tp of tempPositions) {
          if (tp.afterRealId === id) {
            newBlockIds.push(tp.tempId);
          }
        }
      }

      // Prune expired entries and build a set of blocks still in the grace
      // period so we can skip them below in O(1).
      const now = Date.now();
      const graceBlockIds = new Set<number>();
      for (const [blockId, touchedAt] of liveBlockTouched) {
        if (now - touchedAt >= LIVE_BLOCK_GRACE_MS) {
          liveBlockTouched.delete(blockId);
        } else {
          graceBlockIds.add(blockId);
        }
      }

      // Build the new text lines array
      const newTexts: string[] = newBlockIds.map((id) => {
        if (id === lockedBlockId) {
          // Keep the user's local text for the locked block
          const managed = blockData.get(id);
          return managed?.block.text ?? "";
        }
        if (id < 0) {
          // Temp ID: use local text
          const managed = blockData.get(id);
          return managed?.block.text ?? "";
        }
        if (graceBlockIds.has(id)) {
          // Block was recently live-updated — trust the local content
          // (kept up-to-date by the live-block-updated handler) rather
          // than the potentially stale persisted data from the server.
          const managed = blockData.get(id);
          if (managed) return managed.block.text ?? "";
        }
        const fresh = freshMap.get(id);
        return fresh?.block.text ?? "";
      });

      // Update blockData for all fresh blocks, but skip:
      // - our own locked block (we hold the authoritative text)
      // - blocks in the live grace period (live content is fresher)
      for (const fb of freshBlocks) {
        if (fb.id === lockedBlockId) continue;
        if (graceBlockIds.has(fb.id)) {
          // Merge non-text metadata (group_row, author, etc.) from the
          // server while preserving the local live-updated text.
          const managed = blockData.get(fb.id);
          if (managed) {
            blockData.set(fb.id, {
              id: fb.id,
              block: {...fb.block, text: managed.block.text},
            });
            continue;
          }
        }
        blockData.set(fb.id, fb);
      }

      // Remove blockData entries for blocks no longer present on the server
      const removedIds = currentAllIds.filter(
        (id) => id > 0 && !freshIdSet.has(id) && id !== lockedBlockId
      );
      for (const id of removedIds) {
        blockData.delete(id);
        liveInfo.delete(id);
        liveBlockTouched.delete(id);
      }

      // Apply to CM6: replace entire document content in a single transaction.
      // This is the safest approach — no concerns about change ordering.
      if (!editorView) {
        lineMap.initialize(newBlockIds);
        return;
      }

      const desiredText = newTexts.join("\n");
      const doc = editorView.state.doc;
      const currentText = doc.toString();

      if (desiredText !== currentText) {
        editorView.dispatch({
          changes: {from: 0, to: doc.length, insert: desiredText},
          annotations: isRemoteChange.of(true),
        });
      }

      // Update lineMap atomically
      lineMap.initialize(newBlockIds);

      const addedCount = freshIds.filter(
        (id) => !currentRealIdSet.has(id)
      ).length;
      const removedCount = removedIds.length;
      const keptCount = freshIds.filter((id) =>
        currentRealIdSet.has(id)
      ).length;

      console.log(
        "[typst] Sync applied:",
        `+${addedCount} -${removedCount} ~${keptCount}`
      );

      // Refresh decorations after lineMap changes
      pushLockDecorations();
    } catch (err) {
      console.error("[typst] Failed to apply sync update:", err);
    }
  }

  function teardownEventListeners(): void {
    if (unlistenLiveUpdated) {
      unlistenLiveUpdated();
      unlistenLiveUpdated = null;
    }
    if (unlistenLiveReleased) {
      unlistenLiveReleased();
      unlistenLiveReleased = null;
    }
    if (unlistenSyncCompleted) {
      unlistenSyncCompleted();
      unlistenSyncCompleted = null;
    }
    for (const t of pendingReleases.values()) clearTimeout(t);
    pendingReleases.clear();
  }

  // ===== Public API =====

  /**
   * Load a document: fetch blocks, build lineMap, return initial text for CM6.
   *
   * If the document has zero content blocks, creates one empty block so there
   * is always a 1:1 mapping between CM6 lines and blocks.
   */
  async function loadDocument(docId: string): Promise<string> {
    await cleanup();
    currentDocId = docId;

    const [documentData, liveLocks, identityHex] = await Promise.all([
      BlockService.getBlocks(docId),
      BlockService.getDocumentLocks(docId),
      getCurrentIdentityHex(),
    ]);

    myIdentityHex = identityHex;
    setCurrentUserIdentity(identityHex);

    // Release ghost locks from previous sessions
    const myGhostLocks = liveLocks.filter((l) => l.user_id === identityHex);
    if (myGhostLocks.length > 0) {
      await Promise.all(
        myGhostLocks.map((l) =>
          BlockService.releaseLockBlur(docId, l.block_id).catch((e) =>
            console.warn("[typst] Failed to release ghost lock:", e)
          )
        )
      );
    }

    // Build live info lookup from existing locks (skip our own)
    for (const lb of liveLocks) {
      if (lb.user_id === identityHex) continue;
      liveInfo.set(lb.block_id, {
        userId: lb.user_id,
        username: lb.username,
        color: getUserColor(lb.user_id),
        state: lb.locked_at ? "locked" : "focused",
        content: lb.locked_at ? lb.content : undefined,
      });
    }

    // Filter and sort blocks
    const blocks: {id: number; block: Block}[] = [];
    for (const bd of documentData.blocks) {
      if (bd.block.deleted) continue;
      if (bd.block.block_type === "metadata") continue;
      blocks.push({id: bd.block_id, block: bd.block});
    }

    blocks.sort((a, b) => {
      const gA = a.block.group_id ?? "main";
      const gB = b.block.group_id ?? "main";
      if (gA !== gB) return gA.localeCompare(gB);
      const rA = a.block.group_row ?? "0";
      const rB = b.block.group_row ?? "0";
      if (rA !== rB) return rA.localeCompare(rB);
      return a.id - b.id;
    });

    // Handle empty documents: create one initial empty block.
    // CM6 always has at least 1 line, so the lineMap must also have at least
    // 1 entry to maintain the invariant that lineMap.length === doc.lines.
    if (blocks.length === 0) {
      const emptyBlock = await createTypstBlock(0, "", "main", "0");
      const realId = await BlockService.createBlock(docId, emptyBlock);
      const fullBlock: Block = {...emptyBlock, id: realId};
      blocks.push({id: realId, block: fullBlock});
    }

    // Populate blockData and lineMap
    const blockIds: number[] = [];
    for (const b of blocks) {
      blockData.set(b.id, b);
      blockIds.push(b.id);
    }
    lineMap.initialize(blockIds);

    // Setup event listeners
    await setupEventListeners(docId);

    // Build initial text (join all block texts with newlines)
    const initialText = blocks.map((b) => b.block.text ?? "").join("\n");

    console.log(
      "[typst] Document loaded:",
      blocks.length,
      "blocks,",
      initialText.length,
      "chars"
    );

    // Push initial lock decorations (other users' locks loaded from liveLocks)
    // Deferred slightly so the CM6 view is ready
    setTimeout(() => pushLockDecorations(), 0);

    return initialText;
  }

  async function cleanup(): Promise<void> {
    await releaseCurrentLock();
    teardownEventListeners();

    if (liveUpdateTimeout) {
      clearTimeout(liveUpdateTimeout);
      liveUpdateTimeout = null;
    }
    if (persistTimeout) {
      clearTimeout(persistTimeout);
      persistTimeout = null;
    }
    pendingPersist = null;

    // Wait for any pending creations to finish (but don't block indefinitely)
    if (pendingCreations.size > 0) {
      try {
        await Promise.allSettled(pendingCreations.values());
      } catch {
        // Ignore errors during cleanup
      }
    }
    pendingCreations.clear();
    cancelledCreations.clear();

    currentDocId = null;
    editorView = null;
    lineMap.clear();
    blockData.clear();
    liveInfo.clear();
    liveBlockTouched.clear();
    lockedBlockId = null;
  }

  function setView(view: EditorView): void {
    editorView = view;
  }

  /**
   * Handle cursor moving to a new line (1-based CM6 line number).
   * Updates the focus indicator (dotted border on the cursor's line).
   * Lock acquisition is still driven by handleDocChanged — this only
   * tracks focus for the visual indicator.
   */
  function handleCursorLine(lineNumber: number): void {
    const newIdx = lineNumber - 1; // Convert to 0-based
    if (newIdx === focusedLineIndex) return; // No change
    focusedLineIndex = newIdx;
    pushLockDecorations();
  }

  async function handleBlur(): Promise<void> {
    await releaseCurrentLock();
  }

  function isLineLockedByOther(lineIndex: number): boolean {
    const blockId = lineMap.getBlockId(lineIndex);
    if (blockId === undefined) return false;
    const live = liveInfo.get(blockId);
    return live?.state === "locked";
  }

  function getLineLockInfo(lineIndex: number): LiveBlockInfo | null {
    const blockId = lineMap.getBlockId(lineIndex);
    if (blockId === undefined) return null;
    return liveInfo.get(blockId) ?? null;
  }

  /**
   * Flash the given lines to indicate a rejected edit.
   * Dispatches a StateEffect that the decoration extension picks up.
   */
  function flashLockedLines(lineIndices: number[]): void {
    if (!editorView || lineIndices.length === 0) return;
    editorView.dispatch({
      effects: flashLines.of(lineIndices),
    });
  }

  // ===== Build context =====

  const context: TypstEditorContext = {
    loadDocument,
    cleanup,
    setView,
    handleDocChanged,
    handleCursorLine,
    handleBlur,
    isLineLockedByOther,
    getLineLockInfo,
    flashLockedLines,
  };

  setContext(TYPST_CONTEXT_KEY, context);
  return context;
}

// ========== Context Getter ==========

export function getTypstEditorContext(): TypstEditorContext {
  const ctx = getContext<TypstEditorContext>(TYPST_CONTEXT_KEY);
  if (!ctx) {
    throw new Error(
      "getTypstEditorContext must be called within a TypstEditorPanel"
    );
  }
  return ctx;
}
