/**
 * Typst Editor Context
 *
 * Thin frontend bridge between a collaborative Typst text editor (CodeMirror 6)
 * and the backend-managed document state system (`DocumentStateService`).
 *
 * The backend handles:
 * - Block reconstruction and persistence
 * - Lock management (auto-acquire, auto-release, inactivity timeout)
 * - Live update broadcasting and throttling
 * - Sync integration and diff computation
 * - Time-travel (history mode)
 *
 * This context handles:
 * - CM6 ↔ block mapping via the `lineMap` (TypstLineMap)
 * - Translating CM6 `ViewUpdate`s into backend CRUD calls
 * - Receiving block updates from the backend event channel and applying
 *   them to the CM6 editor view
 * - Lock/focus decoration rendering
 * - Fractional indexing for block ordering
 */

import {getContext, setContext} from "svelte";
import {Annotation, StateEffect} from "@codemirror/state";
import type {EditorView, ViewUpdate} from "@codemirror/view";
import {
  DocumentStateService,
  type BlockUpdate,
  type LockInfo,
} from "$lib/services/documentState";
import {createTypstBlock, type Block} from "$lib/services/block";
import {getUserColor} from "$lib/utils/userColors";
import {TypstLineMap} from "./typstLineMap";
import {extractChangeRegions, type ChangeRegion} from "./typstChangeProcessor";
import type {UnlistenFn} from "@tauri-apps/api/event";

// ========== Constants ==========

const TYPST_CONTEXT_KEY = Symbol("typst-editor-context");

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
  lock: LockInfo | null;
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
  // ===== Internal state =====
  let currentDocId: string | null = null;
  let editorView: EditorView | null = null;

  const lineMap = new TypstLineMap();
  const blockData = new Map<number, ManagedBlock>();

  // Event listener cleanup
  let unlistenBlockUpdates: UnlistenFn | null = null;

  // Current focused line (0-based), tracked for focus indicator
  let focusedLineIndex: number | null = null;

  // Track which block we last sent an update for, so we can show the
  // "own-locked" decoration (solid green border). The backend doesn't
  // echo our own lock back — we track it locally for the visual indicator.
  let ownLockedBlockId: number | null = null;

  // Track which block IDs we're currently creating (awaiting backend response).
  // While a creation is in flight, we hold a local placeholder in blockData
  // and lineMap but the backend doesn't know the block ID yet.
  // Key: the index in lineMap where the placeholder lives.
  // Value: promise that resolves to the real block ID.
  // After resolution the caller should update the lineMap slot.
  //
  // Actually, with the new API, createBlock is synchronous-ish (returns real
  // ID immediately via await). But we need to handle the async gap carefully.
  // We track pending creations so we don't try to update/delete a block that
  // hasn't been assigned its real ID yet.

  /**
   * Set of block IDs that were created in the current document load but whose
   * `createBlock` call hasn't completed yet. We use a sentinel negative ID
   * locally (never sent to the backend) and swap it for the real ID on
   * resolution.
   */
  let tempIdCounter = 0;
  function nextTempId(): number {
    return -++tempIdCounter;
  }
  const pendingCreations = new Map<number, Promise<number>>();
  const cancelledCreations = new Set<number>();

  // ===== CM6 helpers =====

  /**
   * Push the current lock/focus state into CM6 via a StateEffect.
   * Called whenever lock state or focusedLineIndex changes.
   *
   * Lock info now comes directly from blockData (populated by backend events),
   * instead of a separate liveInfo map.
   */
  function pushLockDecorations(): void {
    if (!editorView) return;

    const states: LineLockState[] = [];

    // Other users' locks/focuses from blockData
    for (const [blockId, managed] of blockData) {
      if (!managed.lock) continue;
      const lineIdx = lineMap.getLineIndex(blockId);
      if (lineIdx === undefined) continue;
      states.push({
        lineIndex: lineIdx,
        color: getUserColor(managed.lock.userId),
        username: managed.lock.username,
        kind:
          managed.lock.state === "locked" ? "other-locked" : "other-focused",
      });
    }

    // Own locked line (we track this locally since the backend doesn't echo
    // our own lock state back)
    if (ownLockedBlockId !== null) {
      const lineIdx = lineMap.getLineIndex(ownLockedBlockId);
      if (lineIdx !== undefined) {
        states.push({
          lineIndex: lineIdx,
          color: "var(--primary)",
          username: "",
          kind: "own-locked",
        });
      }
    }

    // Own focused line (only if not already showing own-locked on that line)
    if (focusedLineIndex !== null) {
      const ownLockedLineIdx =
        ownLockedBlockId !== null
          ? lineMap.getLineIndex(ownLockedBlockId)
          : undefined;
      if (focusedLineIndex !== ownLockedLineIdx) {
        // Only show the own-focused indicator if the line is not locked by another user
        const blockId = lineMap.getBlockId(focusedLineIndex);
        const managed =
          blockId !== undefined ? blockData.get(blockId) : undefined;
        const isLockedByOther = managed?.lock?.state === "locked";
        if (!isLockedByOther) {
          states.push({
            lineIndex: focusedLineIndex,
            color: "var(--primary)",
            username: "",
            kind: "own-focused",
          });
        }
      }
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

  // ===== Block Update Event Handler =====

  /**
   * Process block updates from the backend event channel.
   *
   * This is the ONLY way block state flows from the backend to the frontend.
   * Handles initial load, live collaboration, sync, and time-travel diffs.
   */
  function handleBlockUpdates(updates: BlockUpdate[]): void {
    if (!editorView) return;

    for (const update of updates) {
      if (update.type === "set") {
        handleBlockSet(update.blockId, update.block, update.lock);
      } else if (update.type === "remove") {
        handleBlockRemove(update.blockId);
      }
    }

    // Refresh decorations after processing all updates
    pushLockDecorations();
  }

  /**
   * Handle a Set event: block was added or content/lock changed.
   */
  function handleBlockSet(
    blockId: number,
    block: Block,
    lock: LockInfo | null
  ): void {
    const existing = blockData.get(blockId);
    const lineIdx = lineMap.getLineIndex(blockId);

    if (existing && lineIdx !== undefined) {
      // Block exists — update content and lock state
      const textChanged = existing.block.text !== block.text;
      blockData.set(blockId, {id: blockId, block, lock});

      if (textChanged && editorView) {
        remoteSetLineText(lineIdx + 1, block.text ?? "");
      }
    } else if (!existing) {
      // New block — need to insert into CM6 and lineMap
      blockData.set(blockId, {id: blockId, block, lock});

      // Find the correct insertion position based on sort order
      const insertIdx = findInsertPosition(blockId, block);

      // Insert into lineMap
      lineMap.splice(insertIdx, 0, blockId);

      // Insert the line into CM6
      if (editorView) {
        const doc = editorView.state.doc;

        if (
          insertIdx === 0 &&
          lineMap.length === 1 &&
          doc.lines === 1 &&
          doc.line(1).text === ""
        ) {
          // Special case: replacing the initial empty document
          remoteSetLineText(1, block.text ?? "");
        } else if (insertIdx >= doc.lines) {
          // Append at end
          const lastLine = doc.line(doc.lines);
          editorView.dispatch({
            changes: {
              from: lastLine.to,
              to: lastLine.to,
              insert: "\n" + (block.text ?? ""),
            },
            annotations: isRemoteChange.of(true),
          });
        } else {
          // Insert before the line at insertIdx
          const targetLine = doc.line(insertIdx + 1);
          editorView.dispatch({
            changes: {
              from: targetLine.from,
              to: targetLine.from,
              insert: (block.text ?? "") + "\n",
            },
            annotations: isRemoteChange.of(true),
          });
        }
      }
    } else {
      // Block exists in blockData but not in lineMap (shouldn't happen normally)
      // Just update the data
      blockData.set(blockId, {id: blockId, block, lock});
    }
  }

  /**
   * Handle a Remove event: block was deleted or no longer exists.
   */
  function handleBlockRemove(blockId: number): void {
    const lineIdx = lineMap.getLineIndex(blockId);
    blockData.delete(blockId);

    if (lineIdx === undefined || !editorView) return;

    const doc = editorView.state.doc;

    // Remove the line from CM6
    if (doc.lines === 1) {
      // Last line — clear it instead of removing
      const line = doc.line(1);
      if (line.text !== "") {
        editorView.dispatch({
          changes: {from: line.from, to: line.to, insert: ""},
          annotations: isRemoteChange.of(true),
        });
      }
    } else if (lineIdx === doc.lines - 1) {
      // Last line — remove the preceding newline too
      const prevLine = doc.line(lineIdx); // lineIdx is 0-based, doc.line is 1-based
      const thisLine = doc.line(lineIdx + 1);
      editorView.dispatch({
        changes: {from: prevLine.to, to: thisLine.to, insert: ""},
        annotations: isRemoteChange.of(true),
      });
    } else {
      // Middle or first line — remove the line and the following newline
      const thisLine = doc.line(lineIdx + 1);
      const nextLine = doc.line(lineIdx + 2);
      editorView.dispatch({
        changes: {from: thisLine.from, to: nextLine.from, insert: ""},
        annotations: isRemoteChange.of(true),
      });
    }

    // Remove from lineMap
    lineMap.splice(lineIdx, 1);
  }

  /**
   * Find the insertion position for a new block based on sort order.
   * Blocks are sorted by (group_id, group_row, block_id).
   */
  function findInsertPosition(blockId: number, block: Block): number {
    const allIds = lineMap.getAllBlockIds();
    const groupId = block.group_id ?? "main";
    const groupRow = block.group_row ?? "0";

    for (let i = 0; i < allIds.length; i++) {
      const existingManaged = blockData.get(allIds[i]);
      if (!existingManaged) continue;

      const eGroupId = existingManaged.block.group_id ?? "main";
      const eGroupRow = existingManaged.block.group_row ?? "0";

      // Compare (group_id, group_row, block_id) lexicographically
      if (eGroupId > groupId) return i;
      if (eGroupId === groupId) {
        if (eGroupRow > groupRow) return i;
        if (eGroupRow === groupRow && allIds[i] > blockId) return i;
      }
    }

    return allIds.length; // Append at end
  }

  // ===== Block CRUD =====

  /**
   * Update an existing block's text via the backend.
   * The backend handles lock acquisition, live broadcast, and persistence.
   */
  function updateBlockText(blockId: number, newText: string): void {
    if (!currentDocId) return;
    const managed = blockData.get(blockId);
    if (!managed) return;

    // Skip if text hasn't actually changed
    if (managed.block.text === newText) return;

    // Track which block we're editing locally (for own-locked decoration)
    if (ownLockedBlockId !== blockId) {
      ownLockedBlockId = blockId;
      pushLockDecorations();
    }

    const updatedBlock: Block = {...managed.block, text: newText};
    blockData.set(blockId, {
      id: blockId,
      block: updatedBlock,
      lock: managed.lock,
    });

    // Send to backend (fire and forget — backend handles everything)
    DocumentStateService.updateBlock(currentDocId, blockId, updatedBlock).catch(
      (err) => {
        console.error("[typst] Failed to update block:", err);
      }
    );
  }

  /**
   * Create a new block asynchronously via the backend.
   *
   * Returns a temp ID immediately for lineMap placement. The async creation
   * fires in the background. When it resolves, the temp ID is swapped for
   * the real ID in both lineMap and blockData.
   *
   * IMPORTANT: Does NOT modify the lineMap. The caller is responsible for
   * inserting the returned temp ID into the lineMap at the correct position.
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
    blockData.set(tempId, {id: tempId, block: placeholderBlock, lock: null});

    // Fire async creation via DocumentStateService
    const promise = (async () => {
      const block = await createTypstBlock(0, text, "main", groupRow);
      const realId = await DocumentStateService.createBlock(docId, block);

      // Check if creation was cancelled (line was deleted before completion)
      if (cancelledCreations.has(tempId)) {
        cancelledCreations.delete(tempId);
        pendingCreations.delete(tempId);
        // Clean up: delete the block we just created on the backend
        try {
          await DocumentStateService.deleteBlock(docId, realId);
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
        blockData.set(realId, {id: realId, block: realBlock, lock: null});

        // If text changed while creation was in flight, send the latest to backend
        if (currentManaged.block.text !== text) {
          try {
            await DocumentStateService.updateBlock(docId, realId, realBlock);
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

    // Clear own-locked if we're deleting the locked block
    if (ownLockedBlockId === blockId) {
      ownLockedBlockId = null;
    }

    const docId = currentDocId;
    DocumentStateService.deleteBlock(docId, blockId).catch((err) => {
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
    lineMap.splice(oldFirstLineIdx, oldLineCount, ...newBlockIds);
  }

  // ===== Public API =====

  /**
   * Load a document: set up event listener, call openDocument, wait for initial
   * state to arrive via the event channel, return initial text for CM6.
   *
   * If the document has zero content blocks, the backend creates one empty
   * block so there is always a 1:1 mapping between CM6 lines and blocks.
   */
  async function loadDocument(docId: string): Promise<string> {
    await cleanup();
    currentDocId = docId;

    // Set up the event listener BEFORE opening the document (per API contract)
    // so we receive the initial state.
    //
    // We collect the initial batch of updates and use them to build the
    // initial CM6 text synchronously after openDocument resolves.
    let initialUpdates: BlockUpdate[] | null = null;
    let resolveInitial: (() => void) | null = null;
    const initialPromise = new Promise<void>((resolve) => {
      resolveInitial = resolve;
    });

    unlistenBlockUpdates = await DocumentStateService.onBlockUpdates(
      docId,
      (updates) => {
        if (initialUpdates === null) {
          // First batch = initial state from openDocument
          initialUpdates = updates;
          resolveInitial?.();
        } else {
          // Subsequent batches = live updates, sync, etc.
          handleBlockUpdates(updates);
        }
      }
    );

    // Open the document on the backend (triggers initial state emission)
    await DocumentStateService.openDocument(docId);

    // Wait for initial state to arrive
    await initialPromise;

    // Process initial state: populate blockData and lineMap
    const blocks: {id: number; block: Block; lock: LockInfo | null}[] = [];
    for (const update of initialUpdates!) {
      if (update.type === "set") {
        blocks.push({
          id: update.blockId,
          block: update.block,
          lock: update.lock,
        });
      }
    }

    // Sort by (group_id, group_row, block_id)
    blocks.sort((a, b) => {
      const gA = a.block.group_id ?? "main";
      const gB = b.block.group_id ?? "main";
      if (gA !== gB) return gA.localeCompare(gB);
      const rA = a.block.group_row ?? "0";
      const rB = b.block.group_row ?? "0";
      if (rA !== rB) return rA.localeCompare(rB);
      return a.id - b.id;
    });

    // Handle empty documents: create one initial empty block
    if (blocks.length === 0) {
      const emptyBlock = await createTypstBlock(0, "", "main", "0");
      const realId = await DocumentStateService.createBlock(docId, emptyBlock);
      const fullBlock: Block = {...emptyBlock, id: realId};
      blocks.push({id: realId, block: fullBlock, lock: null});
    }

    // Populate blockData and lineMap
    const blockIds: number[] = [];
    for (const b of blocks) {
      blockData.set(b.id, {id: b.id, block: b.block, lock: b.lock});
      blockIds.push(b.id);
    }
    lineMap.initialize(blockIds);

    // Build initial text (join all block texts with newlines)
    const initialText = blocks.map((b) => b.block.text ?? "").join("\n");

    console.log(
      "[typst] Document loaded:",
      blocks.length,
      "blocks,",
      initialText.length,
      "chars"
    );

    // Push initial lock decorations after CM6 view is ready
    setTimeout(() => pushLockDecorations(), 0);

    return initialText;
  }

  async function cleanup(): Promise<void> {
    // Close document on backend (releases locks, flushes persists)
    if (currentDocId) {
      try {
        await DocumentStateService.closeDocument(currentDocId);
      } catch (err) {
        console.debug("[typst] Close document failed:", err);
      }
    }

    // Remove event listener
    if (unlistenBlockUpdates) {
      unlistenBlockUpdates();
      unlistenBlockUpdates = null;
    }

    // Wait for any pending creations to finish
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
    focusedLineIndex = null;
    ownLockedBlockId = null;
  }

  function setView(view: EditorView): void {
    editorView = view;
  }

  /**
   * Handle cursor moving to a new line (1-based CM6 line number).
   * Updates the focus indicator (dotted border on the cursor's line).
   */
  function handleCursorLine(lineNumber: number): void {
    const newIdx = lineNumber - 1; // Convert to 0-based
    if (newIdx === focusedLineIndex) return; // No change
    focusedLineIndex = newIdx;
    pushLockDecorations();
  }

  async function handleBlur(): Promise<void> {
    // The backend handles lock release via the inactivity timer and
    // close_document. A blur from the editor doesn't require explicit
    // lock release — the backend will auto-release after 60s of inactivity.
    // Clear local lock/focus indicators.
    ownLockedBlockId = null;
    focusedLineIndex = null;
    pushLockDecorations();
  }

  function isLineLockedByOther(lineIndex: number): boolean {
    const blockId = lineMap.getBlockId(lineIndex);
    if (blockId === undefined) return false;
    const managed = blockData.get(blockId);
    return managed?.lock?.state === "locked";
  }

  function getLineLockInfo(lineIndex: number): LiveBlockInfo | null {
    const blockId = lineMap.getBlockId(lineIndex);
    if (blockId === undefined) return null;
    const managed = blockData.get(blockId);
    if (!managed?.lock) return null;
    return {
      userId: managed.lock.userId,
      username: managed.lock.username,
      color: getUserColor(managed.lock.userId),
      state: managed.lock.state,
    };
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
