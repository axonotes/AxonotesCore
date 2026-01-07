import {invoke} from "@tauri-apps/api/core";

/**
 * Encryption Service
 * Handles all encryption key management operations
 */
export class EncryptionService {
  /**
   * Check if user exists on SpacetimeDB
   * @returns true if user has keys on server
   */
  static async doesUserExist(): Promise<boolean> {
    return await invoke<boolean>("does_stdb_user_exist");
  }

  /**
   * Check if local keys need to be synced from server
   * @returns true if keys don't exist locally (need sync)
   */
  static async doKeysNeedSync(): Promise<boolean> {
    return await invoke<boolean>("do_stdb_keys_need_sync");
  }

  /**
   * Create a new user with encryption keys
   * Generates keys, encrypts them with password, uploads to server
   * @param password - Master password to encrypt keys
   * @returns 12-word mnemonic backup phrase
   */
  static async createUser(password: string): Promise<string> {
    return await invoke<string>("create_stdb_user", {password});
  }

  /**
   * Sync encryption keys from server using master password
   * Downloads encrypted keys, decrypts with password, stores locally
   * @param password - Master password to decrypt keys
   */
  static async syncWithPassword(password: string): Promise<void> {
    await invoke("sync_stdb_keys_with_pwd", {password});
  }

  /**
   * Sync encryption keys from server using mnemonic backup phrase
   * Downloads encrypted keys, decrypts with mnemonic, stores locally
   * @param mnemonic - 12-word backup phrase
   */
  static async syncWithMnemonic(mnemonic: string): Promise<void> {
    await invoke("sync_stdb_keys_with_mnemonic", {mnemonic});
  }

  /**
   * Update master password using old mnemonic
   * Re-encrypts keys with new password, generates NEW mnemonic
   * @param oldMnemonic - Current 12-word backup phrase
   * @param newPassword - New master password
   * @returns NEW 12-word mnemonic (old one becomes invalid!)
   */
  static async updatePasswordFromMnemonic(
    oldMnemonic: string,
    newPassword: string
  ): Promise<string> {
    return await invoke<string>("update_pwd_from_mnemonic", {
      oldMnemonic,
      newPassword,
    });
  }

  /**
   * Update master password using old password
   * Re-encrypts keys with new password (mnemonic unchanged)
   * @param oldPassword - Current master password
   * @param newPassword - New master password
   */
  static async updatePasswordFromPassword(
    oldPassword: string,
    newPassword: string
  ): Promise<void> {
    await invoke("update_pwd_from_pwd", {oldPassword, newPassword});
  }

  /**
   * Generate new mnemonic using current password
   * Re-encrypts keys with new mnemonic (password unchanged)
   * @param password - Current master password
   * @returns NEW 12-word mnemonic (old one becomes invalid!)
   */
  static async updateMnemonicFromPassword(password: string): Promise<string> {
    return await invoke<string>("update_mnemonic_from_pwd", {password});
  }

  /**
   * Generate new mnemonic using old mnemonic
   * Re-encrypts keys with new mnemonic (password unchanged)
   * @param oldMnemonic - Current 12-word backup phrase
   * @returns NEW 12-word mnemonic (old one becomes invalid!)
   */
  static async updateMnemonicFromMnemonic(
    oldMnemonic: string
  ): Promise<string> {
    return await invoke<string>("update_mnemonic_from_mnemonic", {oldMnemonic});
  }
}
