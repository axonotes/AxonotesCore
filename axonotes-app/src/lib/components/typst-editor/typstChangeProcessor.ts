/**
 * Processes CodeMirror 6 document changes and produces line-level operations.
 *
 * Algorithm for each atomic change in a transaction:
 *
 * 1. Count newlines in the OLD range [fromA, toA) to determine oldLineCount
 *    (a range with N newlines spans N+1 lines).
 * 2. Use `inserted.lines` to determine newLineCount
 *    (CM6 Text.lines: "" = 1, "a" = 1, "a\nb" = 2, "\n" = 2).
 * 3. Pair old lines with new lines (first-to-first):
 *    - min(old, new) common lines -> update text
 *    - Extra new lines -> create blocks
 *    - Extra old lines -> delete blocks
 *
 * Change regions are collected from ALL changes in the transaction, then
 * returned in reverse document order so the caller can apply lineMap splices
 * without index shifting issues.
 */

import type {Text, ChangeSet} from "@codemirror/state";

// ========== Types ==========

/**
 * A computed change region describing what happened at the line level.
 */
export interface ChangeRegion {
  /** 0-based line index in the OLD doc where this change starts */
  oldFirstLineIdx: number;
  /** Number of lines affected in the OLD doc */
  oldLineCount: number;
  /** Number of lines produced in the NEW doc for this region */
  newLineCount: number;
  /** Text content for each new line (length === newLineCount) */
  newTexts: string[];
}

// ========== Core ==========

/**
 * Count newline characters (0x0A) in a string.
 */
function countNewlines(text: string): number {
  let count = 0;
  for (let i = 0; i < text.length; i++) {
    if (text.charCodeAt(i) === 10) count++;
  }
  return count;
}

/**
 * Extract change regions from a CM6 ChangeSet.
 *
 * Returns regions in reverse document order (last region first) so that
 * the caller can splice the lineMap sequentially without earlier splices
 * invalidating later indices.
 *
 * @param oldDoc - The document state BEFORE the changes
 * @param newDoc - The document state AFTER the changes
 * @param changes - The CM6 ChangeSet describing the mutations
 * @returns Change regions in reverse document order
 */
export function extractChangeRegions(
  oldDoc: Text,
  newDoc: Text,
  changes: ChangeSet
): ChangeRegion[] {
  const regions: ChangeRegion[] = [];

  changes.iterChanges(
    (
      fromA: number,
      toA: number,
      fromB: number,
      _toB: number,
      inserted: Text
    ) => {
      // --- Old lines affected ---
      // Count newlines in the replaced range to determine how many old lines
      // are consumed. A range with 0 newlines touches 1 line; N newlines = N+1 lines.
      const oldText = fromA < toA ? oldDoc.sliceString(fromA, toA) : "";
      const oldLineCount = 1 + countNewlines(oldText);

      // --- New lines produced ---
      // CM6 Text.lines: number of lines in the inserted content.
      // Empty insertion (pure deletion) counts as 1 line (the surviving partial line).
      const newLineCount = inserted.lines;

      // --- Line numbers ---
      // First affected line in the old doc (convert CM6 1-based to 0-based index).
      const oldFirstLineIdx = oldDoc.lineAt(fromA).number - 1;

      // Read new line texts from the final new doc.
      // fromB is the position in the new doc corresponding to the start of this change.
      const newFirstLineNum = newDoc.lineAt(fromB).number; // 1-based
      const newTexts: string[] = [];
      for (let i = 0; i < newLineCount; i++) {
        newTexts.push(newDoc.line(newFirstLineNum + i).text);
      }

      regions.push({
        oldFirstLineIdx,
        oldLineCount,
        newLineCount,
        newTexts,
      });
    }
  );

  // Reverse so we process from end of document to beginning.
  // This ensures that splicing earlier regions doesn't shift indices
  // of later regions (because later regions come first in the array).
  regions.reverse();

  return regions;
}
