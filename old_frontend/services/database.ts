import {invoke} from "@tauri-apps/api/core";

export type UnlockMode = "none" | "pin" | "pass";

/**
 * Database Service
 * Handles all database-related operations via Tauri commands
 */
export class DatabaseService {
  /**
   * Get unlock mode (reads config file, doesn't need DB unlocked)
   */
  static async getMode(): Promise<UnlockMode> {
    const mode = await invoke<string>("get_unlock_mode");
    return mode as UnlockMode;
  }

  /**
   * Check if database is unlocked (checks global var, doesn't need DB unlocked)
   */
  static async isUnlocked(): Promise<boolean> {
    return await invoke<boolean>("is_database_unlocked");
  }

  /**
   * Unlock the database
   * @param password - Password string, or empty string for mode="none"
   */
  static async unlock(password: string): Promise<void> {
    await invoke("unlock_database", {password});
  }

  /**
   * Set database encryption
   * @param password - New password
   * @param mode - "pin" or "pass"
   */
  static async setEncryption(
    password: string,
    mode: "pin" | "pass"
  ): Promise<void> {
    await invoke("set_database_encryption", {
      newPassword: password,
      mode,
    });
  }

  /**
   * Remove database encryption
   */
  static async removeEncryption(): Promise<void> {
    await invoke("remove_database_encryption");
  }

  /**
   * Switch unlock mode (only between pin ↔ pass)
   */
  static async switchMode(newMode: "pin" | "pass"): Promise<void> {
    await invoke("switch_unlock_mode", {newMode});
  }

  /**
   * Wipe database completely
   */
  static async wipe(): Promise<void> {
    await invoke("wipe_database");
  }
}
