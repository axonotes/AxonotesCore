import {invoke} from '@tauri-apps/api/core';
import type {UnlockMode} from "$lib/types";

export class DatabaseService {
    /**
     * Checks if the database is unlocked
     */
    static async isDatabaseUnlocked(): Promise<boolean> {
        return invoke<boolean>("is_database_unlocked");
    }

    /**
     * Gets the current unlock mode (none | pin | pass)
     */
    static async getCurrentUnlockMode(): Promise<UnlockMode> {
        return invoke<UnlockMode>("get_unlock_mode");
    }

    /**
     * Switches the unlock mode
     * @param newMode The new unlock mode
     */
    static async switchUnlockMode(newMode: UnlockMode): Promise<void> {
        const currentMode = await DatabaseService.getCurrentUnlockMode();

        if (currentMode === newMode) {
            throw new Error(`Already in ${newMode} mode`);
        }

        return invoke("switch_unlock_mode", { newMode });
    }

    /**
     * Unlocks the database
     * @param password The password to unlock the database
     */
    static async unlockDatabase(password: string): Promise<void> {
        return invoke("unlock_database", { password });
    }

    /**
     * Locks the database
     */
    static async lockDatabase(): Promise<void> {
        return invoke("lock_database");
    }

    /**
     * Sets the password and the unlock mode
     * @param newPassword Password to be set
     * @param mode Mode to be set
     */
    static async setDatabaseEncryption(newPassword: string, mode: UnlockMode): Promise<void> {
        return invoke("set_database_encryption", {newPassword, mode});
    }
}