import {invoke} from "@tauri-apps/api/core";

// Raw response from backend (snake_case from Rust)
interface QuotaResponse {
  quota_bytes: number;
  used_bytes: number;
  available_bytes: number;
  matched_rule: string;
}

interface CacheResponse {
  size_bytes: number;
  blob_count: number;
}

// Formatted types for UI
export interface QuotaInfo {
  usedBytes: number;
  totalBytes: number;
  usedFormatted: string;
  totalFormatted: string;
}

export interface CacheInfo {
  sizeBytes: number;
  fileCount: number;
  sizeFormatted: string;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

/**
 * Storage Service
 * Handles storage quota and cache management
 * Note: Storage is auto-initialized by backend when database is unlocked
 */
export class StorageService {
  /**
   * Check if storage is initialized
   */
  static async isInitialized(): Promise<boolean> {
    return await invoke<boolean>("storage_is_initialized");
  }

  /**
   * Get storage quota information
   */
  static async getQuota(): Promise<QuotaInfo> {
    const response = await invoke<QuotaResponse>("get_storage_quota");
    return {
      usedBytes: response.used_bytes,
      totalBytes: response.quota_bytes,
      usedFormatted: formatBytes(response.used_bytes),
      totalFormatted: formatBytes(response.quota_bytes),
    };
  }

  /**
   * Get cache information
   */
  static async getCacheInfo(): Promise<CacheInfo> {
    const response = await invoke<CacheResponse>("get_blob_cache_info");
    return {
      sizeBytes: response.size_bytes,
      fileCount: response.blob_count,
      sizeFormatted: formatBytes(response.size_bytes),
    };
  }

  /**
   * Clear all cached blobs
   */
  static async clearCache(): Promise<void> {
    await invoke("clear_blob_cache");
  }
}
