import {get, writable} from "svelte/store";
import {listen, type UnlistenFn} from "@tauri-apps/api/event";
import {DocumentService, type DocumentMetadata} from "$lib/services/document";

const EXPANDED_FOLDERS_KEY = "axonotes:expanded-folders";

/**
 * Document state
 */
export const documents = writable<DocumentMetadata[]>([]);
export const documentsLoading = writable(false);

/**
 * Temporary folders (in-memory only, disappear if empty after reload)
 */
export const temporaryFolders = writable<Set<string>>(new Set());

/**
 * Selected items (paths) for multi-select functionality
 */
export const selectedPaths = writable<Set<string>>(new Set());

/**
 * Anchor path - the starting point for range selection (set on single select)
 */
export const selectionAnchor = writable<string | null>(null);

/**
 * Current focus path - the current end of range selection (moves with shift+arrow)
 */
export const selectionFocus = writable<string | null>(null);

/**
 * Select a single item (clears other selections, sets new anchor)
 */
export function selectSingle(path: string): void {
  selectedPaths.set(new Set([path]));
  selectionAnchor.set(path);
  selectionFocus.set(path);
}

/**
 * Toggle selection of an item (Ctrl+click)
 */
export function toggleSelection(path: string): void {
  selectedPaths.update((paths) => {
    const newSet = new Set(paths);
    if (newSet.has(path)) {
      newSet.delete(path);
    } else {
      newSet.add(path);
    }
    return newSet;
  });
  // After toggle, set anchor to this item for future range operations
  selectionAnchor.set(path);
  selectionFocus.set(path);
}

/**
 * Select a range of items (Shift+click) - replaces selection with range from anchor
 * @param allPaths Flat list of all visible paths in order
 * @param targetPath The path that was shift+clicked
 */
export function selectRange(allPaths: string[], targetPath: string): void {
  const anchor = get(selectionAnchor);
  if (!anchor) {
    selectSingle(targetPath);
    return;
  }

  const anchorIndex = allPaths.indexOf(anchor);
  const targetIndex = allPaths.indexOf(targetPath);

  if (anchorIndex === -1 || targetIndex === -1) {
    selectSingle(targetPath);
    return;
  }

  const start = Math.min(anchorIndex, targetIndex);
  const end = Math.max(anchorIndex, targetIndex);
  const rangePaths = allPaths.slice(start, end + 1);

  // Replace selection with the range (don't add to existing)
  selectedPaths.set(new Set(rangePaths));
  selectionFocus.set(targetPath);
}

/**
 * Clear all selections
 */
export function clearSelection(): void {
  selectedPaths.set(new Set());
  selectionAnchor.set(null);
  selectionFocus.set(null);
}

/**
 * Check if a path is selected
 */
export function isSelected(path: string): boolean {
  return get(selectedPaths).has(path);
}

/**
 * Expanded folders state (persisted to localStorage)
 */
function loadExpandedFolders(): Set<string> {
  if (typeof localStorage === "undefined") return new Set();
  try {
    const stored = localStorage.getItem(EXPANDED_FOLDERS_KEY);
    if (stored) {
      return new Set(JSON.parse(stored));
    }
  } catch (e) {
    console.error("[documents] Failed to load expanded folders:", e);
  }
  return new Set();
}

function saveExpandedFolders(folders: Set<string>): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(EXPANDED_FOLDERS_KEY, JSON.stringify([...folders]));
  } catch (e) {
    console.error("[documents] Failed to save expanded folders:", e);
  }
}

export const expandedFolders = writable<Set<string>>(loadExpandedFolders());

// Auto-save when expanded folders change
expandedFolders.subscribe((folders) => {
  saveExpandedFolders(folders);
});

/**
 * Toggle a folder's expanded state
 */
export function toggleFolderExpanded(folderPath: string): void {
  expandedFolders.update((folders) => {
    const newSet = new Set(folders);
    if (newSet.has(folderPath)) {
      newSet.delete(folderPath);
    } else {
      newSet.add(folderPath);
    }
    return newSet;
  });
}

/**
 * Set a folder as expanded
 */
export function setFolderExpanded(folderPath: string, expanded: boolean): void {
  expandedFolders.update((folders) => {
    const newSet = new Set(folders);
    if (expanded) {
      newSet.add(folderPath);
    } else {
      newSet.delete(folderPath);
    }
    return newSet;
  });
}

/**
 * Event listeners cleanup functions
 */
let unlisteners: UnlistenFn[] = [];

/**
 * Load all documents from the backend
 */
export async function loadDocuments(): Promise<void> {
  documentsLoading.set(true);
  try {
    const docs = await DocumentService.list();
    documents.set(docs);
  } catch (error) {
    console.error("[documents] Failed to load documents:", error);
    documents.set([]);
  } finally {
    documentsLoading.set(false);
  }
}

/**
 * Create a new document and reload the list
 * @param title Optional title for the document
 * @param folderPath Optional folder path (defaults to "/")
 * @param docType Optional document type: "doc" (default) or "typst"
 * @returns The new document's ID
 */
export async function createDocument(
  title?: string,
  folderPath?: string,
  docType?: string
): Promise<string> {
  const docId = await DocumentService.create(title, folderPath, docType);
  // Backend emits event, which triggers reload
  return docId;
}

/**
 * Delete a document
 * @param docId Document ID to delete
 */
export async function deleteDocument(docId: string): Promise<void> {
  await DocumentService.delete(docId);
  // Backend emits event, which triggers reload
}

/**
 * Rename a document (updates its path, keeping the folder structure)
 * @param docId Document ID
 * @param newName New file name (without path)
 * @throws Error if a document with that name already exists in the same folder
 */
export async function renameDocument(
  docId: string,
  newName: string
): Promise<void> {
  const docs = get(documents);
  const doc = docs.find((d) => d.docId === docId);
  if (!doc) throw new Error("Document not found");

  // Get the folder path and construct new full path
  const parts = doc.path.split("/");
  parts[parts.length - 1] = newName;
  const newPath = parts.join("/");

  // Check if path already exists (excluding current document)
  const existingDoc = docs.find((d) => d.path === newPath && d.docId !== docId);
  if (existingDoc) {
    throw new Error(
      `A document named "${newName}" already exists in this folder`
    );
  }

  await DocumentService.updateMetadata(docId, {
    version: 1,
    path: newPath,
    tags: doc.tags,
  });
  // Backend emits event, which updates local state
}

/**
 * Generate a unique path by adding an index if the path already exists
 * @param basePath The desired path (e.g., "/folder/file.doc")
 * @param existingPaths Set or array of existing paths to check against
 * @returns A unique path (e.g., "/folder/file (1).doc" if original exists)
 */
function getUniquePath(basePath: string, existingPaths: string[]): string {
  if (!existingPaths.includes(basePath)) {
    return basePath;
  }

  // Split into folder, name, and extension
  const lastSlash = basePath.lastIndexOf("/");
  const folder = basePath.substring(0, lastSlash);
  const fullName = basePath.substring(lastSlash + 1);

  // Handle known extensions
  let name: string;
  let ext: string;
  if (fullName.endsWith(".doc")) {
    name = fullName.slice(0, -4);
    ext = ".doc";
  } else if (fullName.endsWith(".typst")) {
    name = fullName.slice(0, -6);
    ext = ".typst";
  } else {
    name = fullName;
    ext = "";
  }

  let counter = 1;
  let newPath: string;
  do {
    newPath = `${folder}/${name} (${counter})${ext}`;
    counter++;
  } while (existingPaths.includes(newPath));

  return newPath;
}

/**
 * Move a document to a new folder
 * @param docId Document ID
 * @param targetFolderPath Target folder path (e.g., "/Work/Projects")
 */
export async function moveDocument(
  docId: string,
  targetFolderPath: string
): Promise<void> {
  const docs = get(documents);
  const doc = docs.find((d) => d.docId === docId);
  if (!doc) throw new Error("Document not found");

  // Get just the filename
  const ext = doc.docType === "typst" ? ".typst" : ".doc";
  const fileName = doc.path.split("/").pop() || `Untitled${ext}`;

  // Construct new path
  const basePath =
    targetFolderPath === "/"
      ? `/${fileName}`
      : `${targetFolderPath}/${fileName}`;

  // Get unique path if there's a conflict
  const existingPaths = docs
    .filter((d) => d.docId !== docId)
    .map((d) => d.path);
  const newPath = getUniquePath(basePath, existingPaths);

  console.log("[documents] Moving document:", {
    docId,
    from: doc.path,
    to: newPath,
    tags: doc.tags,
  });

  try {
    await DocumentService.updateMetadata(docId, {
      version: 1,
      path: newPath,
      tags: doc.tags,
    });
    console.log("[documents] Move successful");
  } catch (error) {
    console.error("[documents] Move failed:", error);
    throw error;
  }

  // Remove temporary folder if a document was moved into it
  // (it's now a real folder since it contains a document)
  temporaryFolders.update((folders) => {
    const newSet = new Set(folders);
    newSet.delete(targetFolderPath);
    console.log(
      "[documents] Removed temporary folder:",
      targetFolderPath,
      "remaining:",
      [...newSet]
    );
    return newSet;
  });
  // Backend emits event, which updates local state
}

/**
 * Generate a unique folder path by adding an index if it already exists
 * @param basePath The desired folder path
 * @param docs Current documents to check for conflicts
 * @param tempFolders Current temporary folders
 * @returns A unique folder path
 */
function getUniqueFolderPath(
  basePath: string,
  docs: DocumentMetadata[],
  tempFolders: Set<string>
): string {
  // Check if any document or temp folder uses this path
  const pathExists = (path: string): boolean => {
    // Check if any document is inside this folder
    if (docs.some((d) => d.path.startsWith(path + "/"))) return true;
    // Check if it's a temporary folder
    if (tempFolders.has(path)) return true;
    return false;
  };

  if (!pathExists(basePath)) {
    return basePath;
  }

  const lastSlash = basePath.lastIndexOf("/");
  const parent = basePath.substring(0, lastSlash);
  const name = basePath.substring(lastSlash + 1);

  let counter = 1;
  let newPath: string;
  do {
    newPath = `${parent}/${name} (${counter})`;
    counter++;
  } while (pathExists(newPath));

  return newPath;
}

/**
 * Move a folder to a new location (updates all documents inside)
 * @param folderPath Current folder path
 * @param targetFolderPath Target parent folder path
 */
export async function moveFolder(
  folderPath: string,
  targetFolderPath: string
): Promise<void> {
  const folderName = folderPath.split("/").filter(Boolean).pop();
  if (!folderName) throw new Error("Invalid folder path");

  const docs = get(documents);
  const tempFolders = get(temporaryFolders);

  // Calculate new folder path with conflict resolution
  const baseFolderPath =
    targetFolderPath === "/"
      ? `/${folderName}`
      : `${targetFolderPath}/${folderName}`;

  const newFolderPath = getUniqueFolderPath(baseFolderPath, docs, tempFolders);

  console.log("[documents] Moving folder:", {
    from: folderPath,
    to: newFolderPath,
  });

  // Update all documents inside this folder
  const docsToUpdate = docs.filter((d) => d.path.startsWith(folderPath + "/"));

  for (const doc of docsToUpdate) {
    const newDocPath = doc.path.replace(folderPath, newFolderPath);
    await DocumentService.updateMetadata(doc.docId, {
      version: 1,
      path: newDocPath,
      tags: doc.tags,
    });
  }

  // Update temporary folder entries
  temporaryFolders.update((folders) => {
    const newSet = new Set<string>();
    for (const folder of folders) {
      if (folder === folderPath) {
        newSet.add(newFolderPath);
      } else if (folder.startsWith(folderPath + "/")) {
        newSet.add(folder.replace(folderPath, newFolderPath));
      } else {
        newSet.add(folder);
      }
    }
    // Remove target if it was temporary (now has content)
    newSet.delete(targetFolderPath);
    return newSet;
  });
}

/**
 * Move multiple items (documents and folders) to a target folder
 * @param paths Array of paths to move
 * @param targetFolderPath Target folder path
 */
export async function moveMultiple(
  paths: string[],
  targetFolderPath: string
): Promise<void> {
  console.log("[documents] Moving multiple items:", {
    paths,
    to: targetFolderPath,
  });

  // Sort paths by depth (shallower first) to avoid moving children before parents
  const sortedPaths = [...paths].sort(
    (a, b) => a.split("/").length - b.split("/").length
  );

  // Filter out paths that are children of other selected paths (they'll move with parent)
  const topLevelPaths = sortedPaths.filter(
    (path) =>
      !sortedPaths.some(
        (other) => other !== path && path.startsWith(other + "/")
      )
  );

  const docs = get(documents);

  for (const path of topLevelPaths) {
    // Check if it's a document or folder
    const doc = docs.find((d) => d.path === path);

    if (doc) {
      // It's a document
      await moveDocument(doc.docId, targetFolderPath);
    } else {
      // It's a folder
      await moveFolder(path, targetFolderPath);
    }
  }

  // Clear selection after move
  clearSelection();
}

/**
 * Create a temporary folder (in-memory only)
 * @param folderPath Full folder path (e.g., "/Work/NewFolder")
 */
export function createTemporaryFolder(folderPath: string): void {
  console.log("[documents] Creating temporary folder:", folderPath);
  temporaryFolders.update((folders) => {
    const newSet = new Set(folders);
    newSet.add(folderPath);
    console.log("[documents] Temporary folders now:", [...newSet]);
    return newSet;
  });
}

/**
 * Remove a temporary folder
 * @param folderPath Folder path to remove
 */
export function removeTemporaryFolder(folderPath: string): void {
  temporaryFolders.update((folders) => {
    const newSet = new Set(folders);
    newSet.delete(folderPath);
    return newSet;
  });
}

/**
 * Rename a folder (updates paths of all documents inside and temporary folder entry)
 * @param oldPath Current folder path
 * @param newName New folder name (just the name, not the full path)
 * @throws Error if a folder with that name already exists at the same level
 */
export async function renameFolder(
  oldPath: string,
  newName: string
): Promise<void> {
  // Calculate new path
  const pathParts = oldPath.split("/").filter(Boolean);
  pathParts[pathParts.length - 1] = newName;
  const newPath = "/" + pathParts.join("/");

  // Check if folder already exists (has documents inside or is a temporary folder)
  const docs = get(documents);
  const tempFolders = get(temporaryFolders);

  const folderExists =
    docs.some((d) => d.path.startsWith(newPath + "/")) ||
    tempFolders.has(newPath);

  if (folderExists && newPath !== oldPath) {
    throw new Error(
      `A folder named "${newName}" already exists at this location`
    );
  }

  console.log("[documents] Renaming folder:", {oldPath, newPath});

  // Update all documents that are inside this folder
  const docsToUpdate = docs.filter(
    (d) => d.path.startsWith(oldPath + "/") || d.path === oldPath
  );

  for (const doc of docsToUpdate) {
    const newDocPath = doc.path.replace(oldPath, newPath);
    console.log("[documents] Updating doc path:", {
      docId: doc.docId,
      from: doc.path,
      to: newDocPath,
    });
    await DocumentService.updateMetadata(doc.docId, {
      version: 1,
      path: newDocPath,
      tags: doc.tags,
    });
  }

  // Update temporary folder if it exists
  temporaryFolders.update((folders) => {
    const newSet = new Set<string>();
    for (const folder of folders) {
      if (folder === oldPath) {
        newSet.add(newPath);
      } else if (folder.startsWith(oldPath + "/")) {
        // Subfolder of renamed folder
        newSet.add(folder.replace(oldPath, newPath));
      } else {
        newSet.add(folder);
      }
    }
    return newSet;
  });
}

/**
 * Setup event listeners for document changes
 * Call this once when the app initializes
 */
export async function setupDocumentListeners(): Promise<void> {
  // Clean up any existing listeners
  await cleanupDocumentListeners();

  // Listen for document created
  const unlistenCreated = await listen<{docId: string}>(
    "document-created",
    async () => {
      console.log("[documents] Document created, reloading list");
      await loadDocuments();
    }
  );

  // Listen for document deleted
  const unlistenDeleted = await listen<{docId: string}>(
    "document-deleted",
    async (event) => {
      console.log("[documents] Document deleted:", event.payload.docId);
      // Remove from local state immediately
      documents.update((docs) =>
        docs.filter((d) => d.docId !== event.payload.docId)
      );
    }
  );

  // Listen for metadata updated
  const unlistenMetadata = await listen<{
    docId: string;
    path: string;
    tags: string[];
    docType: string;
  }>("document-metadata-updated", async (event) => {
    console.log("[documents] Metadata updated event received:", event.payload);
    // Update local state immediately
    documents.update((docs) => {
      const newDocs = docs.map((d) =>
        d.docId === event.payload.docId
          ? {
              ...d,
              path: event.payload.path,
              tags: event.payload.tags,
              docType: event.payload.docType ?? d.docType,
            }
          : d
      );
      console.log(
        "[documents] Updated docs paths:",
        newDocs.map((d) => d.path)
      );
      return newDocs;
    });
  });

  // Listen for access granted (shared documents)
  const unlistenAccessGranted = await listen<{docId: string}>(
    "document-access-granted",
    async () => {
      console.log("[documents] Access granted, reloading list");
      await loadDocuments();
    }
  );

  // Listen for access revoked
  const unlistenAccessRevoked = await listen<{docId: string}>(
    "document-access-revoked",
    async (event) => {
      console.log("[documents] Access revoked:", event.payload.docId);
      // Remove from local state immediately
      documents.update((docs) =>
        docs.filter((d) => d.docId !== event.payload.docId)
      );
    }
  );

  unlisteners = [
    unlistenCreated,
    unlistenDeleted,
    unlistenMetadata,
    unlistenAccessGranted,
    unlistenAccessRevoked,
  ];
}

/**
 * Cleanup event listeners
 */
export async function cleanupDocumentListeners(): Promise<void> {
  for (const unlisten of unlisteners) {
    unlisten();
  }
  unlisteners = [];
}

// ============================================================================
// Tree Structure Helpers
// ============================================================================

/**
 * Tree node for file/folder hierarchy
 */
export interface TreeNode {
  name: string;
  type: "folder" | "file";
  path: string;
  docId?: string; // Only for files
  isTemporary?: boolean; // Only for temporary folders
  children: TreeNode[];
}

/**
 * Build a tree structure from flat document list and temporary folders
 * @param docs Document metadata list
 * @param tempFolders Set of temporary folder paths
 * @returns Root tree nodes
 */
export function buildTree(
  docs: DocumentMetadata[],
  tempFolders: Set<string> = new Set()
): TreeNode[] {
  console.log(
    "[buildTree] Building tree with",
    docs.length,
    "docs and",
    tempFolders.size,
    "temp folders"
  );
  console.log(
    "[buildTree] Docs:",
    docs.map((d) => d.path)
  );
  console.log("[buildTree] TempFolders:", [...tempFolders]);

  // Map of path -> TreeNode for quick lookup
  const nodeMap = new Map<string, TreeNode>();

  // Helper to get or create a folder node at a path
  const getOrCreateFolder = (
    folderPath: string,
    isTemporary = false
  ): TreeNode => {
    if (nodeMap.has(folderPath)) {
      return nodeMap.get(folderPath)!;
    }

    const parts = folderPath.split("/").filter(Boolean);
    const name = parts[parts.length - 1];

    const node: TreeNode = {
      name,
      type: "folder",
      path: folderPath,
      isTemporary,
      children: [],
    };

    nodeMap.set(folderPath, node);
    return node;
  };

  // Helper to get parent path
  const getParentPath = (path: string): string | null => {
    const parts = path.split("/").filter(Boolean);
    if (parts.length <= 1) return null;
    return "/" + parts.slice(0, -1).join("/");
  };

  // Step 1: Create all temporary folders
  for (const folderPath of tempFolders) {
    getOrCreateFolder(folderPath, true);

    // Also create parent folders if needed
    let parentPath = getParentPath(folderPath);
    while (parentPath) {
      getOrCreateFolder(parentPath, false);
      parentPath = getParentPath(parentPath);
    }
  }

  // Step 2: Create all documents and their parent folders
  for (const doc of docs) {
    const parts = doc.path.split("/").filter(Boolean);
    const fileName = parts[parts.length - 1];

    // Create the file node
    const fileNode: TreeNode = {
      name: fileName,
      type: "file",
      path: doc.path,
      docId: doc.docId,
      children: [],
    };
    nodeMap.set(doc.path, fileNode);

    // Create parent folders
    let parentPath = getParentPath(doc.path);
    while (parentPath) {
      getOrCreateFolder(parentPath, false);
      parentPath = getParentPath(parentPath);
    }
  }

  // Step 3: Build the tree by linking children to parents
  const root: TreeNode[] = [];

  for (const node of nodeMap.values()) {
    const parentPath = getParentPath(node.path);

    if (parentPath === null) {
      // Root level node
      root.push(node);
    } else {
      // Has a parent - add to parent's children
      const parent = nodeMap.get(parentPath);
      if (parent) {
        parent.children.push(node);
      } else {
        // Parent doesn't exist (shouldn't happen), add to root
        console.warn(`[buildTree] Parent not found for ${node.path}`);
        root.push(node);
      }
    }
  }

  // Step 4: Sort everything - folders first, then alphabetically
  const sortNodes = (nodes: TreeNode[]) => {
    nodes.sort((a, b) => {
      if (a.type !== b.type) return a.type === "folder" ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    nodes.forEach((n) => sortNodes(n.children));
  };
  sortNodes(root);

  console.log("[buildTree] Result:", root.length, "root nodes");
  const logTree = (nodes: TreeNode[], indent = "") => {
    for (const n of nodes) {
      console.log(
        `[buildTree] ${indent}${n.type === "folder" ? "📁" : "📄"} ${n.name} (${n.path})`
      );
      if (n.children.length > 0) logTree(n.children, indent + "  ");
    }
  };
  logTree(root);

  return root;
}

/**
 * Get a document by ID from the current store
 */
export function getDocumentById(docId: string): DocumentMetadata | undefined {
  return get(documents).find((d) => d.docId === docId);
}

/**
 * Flatten a tree into a list of paths (in display order)
 * Only includes visible items (expanded folders' children)
 */
export function flattenTreePaths(
  nodes: TreeNode[],
  expandedFolders: Set<string>
): string[] {
  const paths: string[] = [];

  const traverse = (nodeList: TreeNode[]) => {
    for (const node of nodeList) {
      paths.push(node.path);
      if (node.type === "folder" && expandedFolders.has(node.path)) {
        traverse(node.children);
      }
    }
  };

  traverse(nodes);
  return paths;
}

/**
 * Delete multiple items (documents and folders)
 * For folders, all documents inside are deleted first
 * @param paths Array of paths to delete
 * @returns Object with deleted count and any errors
 */
export async function deleteMultiple(
  paths: string[]
): Promise<{deleted: number; errors: string[]}> {
  const docs = get(documents);
  const errors: string[] = [];
  let deleted = 0;

  // Sort paths by depth (deeper first) so we delete children before parents
  // But also separate docs from folders to handle folders specially
  const sortedPaths = [...paths].sort(
    (a, b) => b.split("/").length - a.split("/").length
  );

  // Filter out paths that are children of other selected paths (they'll be deleted with parent folder)
  const topLevelPaths = sortedPaths.filter(
    (path) =>
      !sortedPaths.some(
        (other) => other !== path && path.startsWith(other + "/")
      )
  );

  for (const path of topLevelPaths) {
    const doc = docs.find((d) => d.path === path);
    if (doc) {
      // It's a document - delete it
      try {
        await DocumentService.delete(doc.docId);
        deleted++;
      } catch (e) {
        errors.push(`Failed to delete ${path}: ${e}`);
      }
    } else {
      // It's a folder - delete all documents inside it first
      const docsInFolder = docs.filter((d) => d.path.startsWith(path + "/"));

      for (const folderDoc of docsInFolder) {
        try {
          await DocumentService.delete(folderDoc.docId);
          deleted++;
        } catch (e) {
          errors.push(`Failed to delete ${folderDoc.path}: ${e}`);
        }
      }

      // Remove temporary folder entry if it exists
      const isTemp = get(temporaryFolders).has(path);
      if (isTemp) {
        removeTemporaryFolder(path);
      }

      // Also remove any child temporary folders
      temporaryFolders.update((folders) => {
        const newSet = new Set<string>();
        for (const folder of folders) {
          if (!folder.startsWith(path + "/") && folder !== path) {
            newSet.add(folder);
          }
        }
        return newSet;
      });

      // Count the folder itself as deleted if it had content or was temporary
      if (docsInFolder.length > 0 || isTemp) {
        deleted++;
      }
    }
  }

  clearSelection();
  return {deleted, errors};
}
