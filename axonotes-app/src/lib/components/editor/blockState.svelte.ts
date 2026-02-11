import {
  BlockService,
  type Block,
  type DecryptedLiveBlock,
} from "$lib/services/block";
import {getUserColor, getOwnColor} from "$lib/utils/userColors";
import type {BlockEventHandler} from "./documentContext";

// ========== Constants ==========

const LIVE_UPDATE_THROTTLE_MS = 100;
const PERSIST_DEBOUNCE_MS = 300;
const RELEASE_WINDOW_MS = 50;

// ========== Types ==========

export interface LiveUser {
  userId: string;
  username: string;
  color: string;
}

export type BlockState =
  | {type: "normal"}
  | {type: "focused"}
  | {type: "editing"}
  | {type: "locked"; user: LiveUser};

export interface BorderStyle {
  show: boolean;
  color: string;
  style: "solid" | "dotted";
}

export interface BlockStateManager extends BlockEventHandler {
  // Reactive state (read via getters)
  readonly state: BlockState;
  readonly content: Block;
  readonly displayContent: Block;
  readonly isReadOnly: boolean;
  readonly borderStyle: BorderStyle;
  readonly lockedBy: LiveUser | null;

  // Actions
  focus(): void;
  blur(): Promise<void>;
  startEditing(initialUpdates?: Partial<Block>): Promise<boolean>;
  updateContent(updates: Partial<Block>): void;

  // Lifecycle
  cleanup(): void;
}

// ========== Factory Function ==========

export function createBlockState(
  docId: string,
  blockId: number,
  initialContent: Block,
  username: string,
  onContentPersisted?: (blockId: number, content: Block) => void
): BlockStateManager {
  // ===== Reactive State =====

  let state = $state<BlockState>({type: "normal"});
  let content = $state<Block>(initialContent);
  let remoteContent = $state<Block | null>(null);

  // ===== Derived State =====

  const displayContent = $derived.by(() => {
    if (state.type === "locked" && remoteContent) {
      return remoteContent;
    }
    return content;
  });

  const isReadOnly = $derived(state.type === "locked");

  const borderStyle = $derived.by((): BorderStyle => {
    const ownColor = getOwnColor();

    switch (state.type) {
      case "normal":
        return {show: false, color: "transparent", style: "dotted"};
      case "focused":
        return {show: true, color: ownColor, style: "dotted"};
      case "editing":
        return {show: true, color: ownColor, style: "solid"};
      case "locked":
        return {show: true, color: state.user.color, style: "solid"};
    }
  });

  const lockedBy = $derived(state.type === "locked" ? state.user : null);

  // ===== Private State =====

  let liveUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let persistTimeout: ReturnType<typeof setTimeout> | null = null;
  let pendingReleaseTimeout: ReturnType<typeof setTimeout> | null = null;

  // ===== Remote Event Handlers =====
  // These are called by the document context when remote events arrive

  function handleRemoteUpdate(liveBlock: DecryptedLiveBlock): void {
    // Ignore updates while we're editing (we're the source)
    if (state.type === "editing") {
      return;
    }

    // Cancel any pending release (sliding window)
    if (pendingReleaseTimeout) {
      clearTimeout(pendingReleaseTimeout);
      pendingReleaseTimeout = null;
    }

    // Transition to locked state
    const user: LiveUser = {
      userId: liveBlock.user_id,
      username: liveBlock.username,
      color: getUserColor(liveBlock.user_id),
    };

    if (liveBlock.locked_at) {
      state = {type: "locked", user};
      remoteContent = liveBlock.content;
    } else {
      // Focused but not locked - still show indicator
      state = {type: "locked", user};
      remoteContent = null;
    }
  }

  function handleRemoteRelease(): void {
    // Ignore if we're currently editing
    if (state.type === "editing") {
      return;
    }

    // Clear any existing pending release
    if (pendingReleaseTimeout) {
      clearTimeout(pendingReleaseTimeout);
    }

    // Sliding window: wait briefly for potential re-lock
    pendingReleaseTimeout = setTimeout(() => {
      pendingReleaseTimeout = null;
      if (state.type === "locked") {
        // Copy remote content to local content for UI continuity
        if (remoteContent) {
          content = remoteContent;
        }
        state = {type: "normal"};
        remoteContent = null;
      }
    }, RELEASE_WINDOW_MS);
  }

  // ===== Timers =====

  function cancelLiveUpdateTimer(): void {
    if (liveUpdateTimeout) {
      clearTimeout(liveUpdateTimeout);
      liveUpdateTimeout = null;
    }
  }

  function scheduleLiveUpdate(block: Block): void {
    if (liveUpdateTimeout) return; // Already scheduled

    liveUpdateTimeout = setTimeout(async () => {
      liveUpdateTimeout = null;
      if (state.type !== "editing") return;

      try {
        await BlockService.updateLiveBlock(docId, blockId, block);
      } catch (err) {
        console.error("[blockState] Failed to broadcast live update:", err);
      }
    }, LIVE_UPDATE_THROTTLE_MS);
  }

  function cancelPersistTimer(): void {
    if (persistTimeout) {
      clearTimeout(persistTimeout);
      persistTimeout = null;
    }
  }

  function schedulePersist(block: Block): void {
    cancelPersistTimer();

    persistTimeout = setTimeout(async () => {
      persistTimeout = null;

      try {
        await BlockService.updateBlock(docId, blockId, block);
        onContentPersisted?.(blockId, block);
      } catch (err) {
        console.error("[blockState] Failed to persist block:", err);
      }
    }, PERSIST_DEBOUNCE_MS);
  }

  async function flushPersist(): Promise<void> {
    if (!persistTimeout) return;

    cancelPersistTimer();

    try {
      await BlockService.updateBlock(docId, blockId, content);
      onContentPersisted?.(blockId, content);
    } catch (err) {
      console.error("[blockState] Failed to flush persist:", err);
    }
  }

  // ===== Lock Management =====

  async function acquireLock(): Promise<boolean> {
    try {
      await BlockService.requestLock(docId, blockId, content, username);
      return true;
    } catch (err) {
      console.error("[blockState] Failed to acquire lock:", err);
      return false;
    }
  }

  async function releaseLock(): Promise<void> {
    cancelLiveUpdateTimer();
    await flushPersist();

    try {
      await BlockService.releaseLockBlur(docId, blockId);
    } catch (err) {
      console.debug(
        "[blockState] Lock release failed (may already be released):",
        err
      );
    }
  }

  // ===== Public Actions =====

  function focus(): void {
    if (state.type === "normal") {
      state = {type: "focused"};
    }
  }

  async function blur(): Promise<void> {
    // IMPORTANT: Set state BEFORE async operations to prevent double-blur race
    const wasEditing = state.type === "editing";
    state = {type: "normal"};

    if (wasEditing) {
      await releaseLock();
    }
  }

  async function startEditing(
    initialUpdates?: Partial<Block>
  ): Promise<boolean> {
    // Can only start editing from focused or normal state
    if (state.type === "locked") {
      return false;
    }

    // Store previous state for rollback
    const previousState = state;
    const previousContent = content;

    // Apply initial content updates BEFORE acquiring lock
    if (initialUpdates) {
      content = {...content, ...initialUpdates};
    }

    // IMPORTANT: Set state to "editing" BEFORE the async call
    // This prevents race conditions where multiple calls could pass the check
    state = {type: "editing"};

    const gotLock = await acquireLock();
    if (!gotLock) {
      // Rollback state and content
      state = previousState;
      content = previousContent;
      return false;
    }

    // If we had initial updates, schedule persist for the new content
    if (initialUpdates) {
      schedulePersist(content);
    }

    return true;
  }

  function updateContent(updates: Partial<Block>): void {
    if (state.type !== "editing") {
      console.warn("[blockState] updateContent called outside editing state");
      return;
    }

    content = {...content, ...updates};
    scheduleLiveUpdate(content);
    schedulePersist(content);
  }

  // ===== Cleanup =====

  function cleanup(): void {
    // Release lock if we're editing (fire and forget on cleanup)
    if (state.type === "editing") {
      // Set state first to prevent any further operations
      state = {type: "normal"};
      releaseLock().catch((err) => {
        console.debug("[blockState] Cleanup lock release failed:", err);
      });
    }

    cancelLiveUpdateTimer();
    cancelPersistTimer();
    if (pendingReleaseTimeout) {
      clearTimeout(pendingReleaseTimeout);
      pendingReleaseTimeout = null;
    }
  }

  // ===== Return Manager =====

  return {
    get state() {
      return state;
    },
    get content() {
      return content;
    },
    get displayContent() {
      return displayContent;
    },
    get isReadOnly() {
      return isReadOnly;
    },
    get borderStyle() {
      return borderStyle;
    },
    get lockedBy() {
      return lockedBy;
    },
    focus,
    blur,
    startEditing,
    updateContent,
    cleanup,
    handleRemoteUpdate,
    handleRemoteRelease,
  };
}
