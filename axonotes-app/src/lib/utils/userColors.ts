/**
 * Global user color management for collaborative editing.
 *
 * Colors are assigned consistently across all documents:
 * - Current user always gets the primary/accent color
 * - Other users get randomly assigned colors from a theme-matching palette
 * - Colors persist for the session (same user = same color)
 */

import {getCurrentIdentityHex} from "$lib/services/block";

// Theme-matching collaborator colors
// These are carefully selected to be:
// - Distinct from each other
// - Readable against both light and dark backgrounds
// - Harmonious with the app's design language
const COLLABORATOR_COLORS = [
  "#10B981", // Emerald green
  "#F59E0B", // Amber
  "#EF4444", // Red
  "#8B5CF6", // Violet
  "#EC4899", // Pink
  "#06B6D4", // Cyan
  "#F97316", // Orange
  "#84CC16", // Lime
  "#6366F1", // Indigo
  "#14B8A6", // Teal
  "#A855F7", // Purple
  "#22C55E", // Green
] as const;

// CSS variable for primary color (used for current user)
// Uses oklch format matching the app's theme
const PRIMARY_COLOR = "var(--primary)";
// Fallback if CSS variable not available (blue)
const PRIMARY_COLOR_FALLBACK = "#3B82F6";

// Global map of user IDs to assigned colors
const userColorMap = new Map<string, string>();

// Track which colors have been assigned to avoid duplicates when possible
let colorIndex = 0;

// Cached current user identity
let currentUserHex: string | null = null;

/**
 * Initialize the current user's identity for color assignment.
 * Call this once when the app/editor loads.
 */
export async function initUserColors(): Promise<void> {
  try {
    currentUserHex = await getCurrentIdentityHex();
  } catch {
    currentUserHex = null;
  }
}

/**
 * Get a color for a user ID.
 *
 * @param userId - The user's identity as a hex string
 * @returns A CSS color string
 *
 * - Current user: Primary color (accent)
 * - Other users: Randomly assigned from palette, consistent per session
 */
export function getUserColor(userId: string): string {
  // Current user gets primary color
  if (currentUserHex && userId === currentUserHex) {
    // Try to use CSS variable, fall back to hex
    if (typeof document !== "undefined") {
      return PRIMARY_COLOR;
    }
    return PRIMARY_COLOR_FALLBACK;
  }

  // Check if we already assigned a color to this user
  if (userColorMap.has(userId)) {
    return userColorMap.get(userId)!;
  }

  // Assign a new color, trying to avoid duplicates
  const usedColors = new Set(userColorMap.values());
  let color: string;

  // First try to find an unused color
  const unusedColor = COLLABORATOR_COLORS.find((c) => !usedColors.has(c));
  if (unusedColor) {
    color = unusedColor;
  } else {
    // All colors used, cycle through them
    color = COLLABORATOR_COLORS[colorIndex % COLLABORATOR_COLORS.length];
    colorIndex++;
  }

  userColorMap.set(userId, color);
  return color;
}

/**
 * Get the current user's color (primary/accent color).
 */
export function getOwnColor(): string {
  if (typeof document !== "undefined") {
    return PRIMARY_COLOR;
  }
  return PRIMARY_COLOR_FALLBACK;
}

/**
 * Clear all assigned colors.
 * Call this on logout or when switching profiles.
 */
export function clearUserColors(): void {
  userColorMap.clear();
  colorIndex = 0;
  currentUserHex = null;
}

/**
 * Set the current user's identity directly.
 * Useful when identity is already known.
 */
export function setCurrentUserIdentity(userHex: string): void {
  currentUserHex = userHex;
}
