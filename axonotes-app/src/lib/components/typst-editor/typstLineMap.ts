/**
 * Bidirectional mapping between CodeMirror line indices (0-based) and block IDs.
 *
 * This is the core bridge between CM6's document model (lines) and our
 * backend's block model (TypstV1 blocks with unique IDs).
 *
 * Invariant: lineToBlock.length should equal the number of lines in the CM6
 * document that have associated blocks. Lines without blocks (e.g. the empty
 * line in a brand-new document) are simply absent from the map.
 */
export class TypstLineMap {
  /** Line index (0-based) -> block ID */
  private lineToBlock: number[] = [];

  /** Block ID -> line index (0-based) */
  private blockToLine: Map<number, number> = new Map();

  // ========== Queries ==========

  /** Number of mapped lines */
  get length(): number {
    return this.lineToBlock.length;
  }

  /** Get block ID for a 0-based line index. Returns undefined if unmapped. */
  getBlockId(lineIndex: number): number | undefined {
    return this.lineToBlock[lineIndex];
  }

  /** Get 0-based line index for a block ID. Returns undefined if not found. */
  getLineIndex(blockId: number): number | undefined {
    return this.blockToLine.get(blockId);
  }

  /** Get a slice of block IDs for a contiguous line range. */
  getBlockIds(startLine: number, count: number): (number | undefined)[] {
    const result: (number | undefined)[] = [];
    for (let i = 0; i < count; i++) {
      result.push(this.lineToBlock[startLine + i]);
    }
    return result;
  }

  /** Get all block IDs in line order. */
  getAllBlockIds(): number[] {
    return [...this.lineToBlock];
  }

  /** Whether a block ID exists in the map. */
  hasBlock(blockId: number): boolean {
    return this.blockToLine.has(blockId);
  }

  /** Whether a line index has an associated block. */
  hasLine(lineIndex: number): boolean {
    return lineIndex >= 0 && lineIndex < this.lineToBlock.length;
  }

  // ========== Mutations ==========

  /**
   * Initialize from an ordered array of block IDs.
   * Replaces all existing mappings.
   */
  initialize(blockIds: number[]): void {
    this.lineToBlock = [...blockIds];
    this.rebuildReverse();
  }

  /**
   * Splice the line map: remove `deleteCount` entries starting at `startLine`,
   * then insert `newBlockIds` at that position.
   *
   * Returns the removed block IDs.
   *
   * This mirrors Array.prototype.splice and is the primary mutation method
   * used by the change processor.
   */
  splice(
    startLine: number,
    deleteCount: number,
    ...newBlockIds: number[]
  ): number[] {
    const removed = this.lineToBlock.splice(
      startLine,
      deleteCount,
      ...newBlockIds
    );
    this.rebuildReverse();
    return removed;
  }

  /** Replace the block ID at a specific line index. */
  setBlockId(lineIndex: number, blockId: number): void {
    const old = this.lineToBlock[lineIndex];
    this.lineToBlock[lineIndex] = blockId;
    // Update reverse map
    if (old !== undefined) {
      this.blockToLine.delete(old);
    }
    this.blockToLine.set(blockId, lineIndex);
  }

  /** Clear all mappings. */
  clear(): void {
    this.lineToBlock = [];
    this.blockToLine.clear();
  }

  // ========== Internal ==========

  /** Rebuild the reverse map from the forward map. */
  private rebuildReverse(): void {
    this.blockToLine.clear();
    for (let i = 0; i < this.lineToBlock.length; i++) {
      const blockId = this.lineToBlock[i];
      if (blockId !== undefined) {
        this.blockToLine.set(blockId, i);
      }
    }
  }
}
