import {invoke} from "@tauri-apps/api/core";

/**
 * Document metadata as returned by the backend
 */
interface DocumentMetadataResponse {
  meta_id: string;
  user_id: unknown; // Identity object, not needed in frontend
  doc_id: string;
  metadata: {
    version: number;
    path: string;
    tags: string[];
    doc_type: string; // "doc" or "typst"
  };
}

/**
 * Simplified document metadata for frontend use
 */
export interface DocumentMetadata {
  docId: string;
  path: string;
  tags: string[];
  docType: string; // "doc" or "typst"
}

/**
 * Metadata update payload.
 * Note: doc_type is intentionally omitted — it is immutable after creation.
 * The backend preserves the existing doc_type on every update.
 */
export interface MetadataUpdate {
  version: number;
  path: string;
  tags: string[];
}

/**
 * Document Service
 * Wraps Tauri commands for document operations
 */
export class DocumentService {
  /**
   * List all documents accessible to the current user
   */
  static async list(): Promise<DocumentMetadata[]> {
    const response = await invoke<DocumentMetadataResponse[]>("list_documents");
    return response.map((doc) => ({
      docId: doc.doc_id,
      path: doc.metadata.path,
      tags: doc.metadata.tags,
      docType: doc.metadata.doc_type ?? "doc",
    }));
  }

  /**
   * Create a new document
   * @param title Optional title for the document (defaults to "Default")
   * @param folderPath Optional folder path (defaults to "/")
   * @param docType Optional document type: "doc" (default) or "typst"
   * @returns The new document's ID
   */
  static async create(
    title?: string,
    folderPath?: string,
    docType?: string
  ): Promise<string> {
    return invoke<string>("create_document", {title, folderPath, docType});
  }

  /**
   * Get metadata for a specific document
   * @param docId Document ID
   * @returns Document metadata or null if not found
   */
  static async getMeta(docId: string): Promise<DocumentMetadata | null> {
    const response = await invoke<DocumentMetadataResponse | null>(
      "get_document_meta",
      {docId}
    );
    if (!response) return null;
    return {
      docId: response.doc_id,
      path: response.metadata.path,
      tags: response.metadata.tags,
      docType: response.metadata.doc_type ?? "doc",
    };
  }

  /**
   * Delete a document (owner only)
   * @param docId Document ID to delete
   */
  static async delete(docId: string): Promise<void> {
    await invoke("delete_document", {docId});
  }

  /**
   * Update document metadata (path and tags)
   * @param docId Document ID
   * @param metadata New metadata values
   */
  static async updateMetadata(
    docId: string,
    metadata: MetadataUpdate
  ): Promise<void> {
    await invoke("update_document_metadata", {docId, metadata});
  }
}
