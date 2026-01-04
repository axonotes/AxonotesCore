import { invoke } from '@tauri-apps/api/core';

export type UnlockMode = 'none' | 'pin' | 'pass';

export async function isDatabaseUnlocked(): Promise<boolean> {
    return invoke<boolean>("is_database_unlocked");
}

export async function getCurrentUnlockMode(): Promise<UnlockMode> {
    return invoke<UnlockMode>("get_unlock_mode");
}

export async function switchUnlockMode(newMode: UnlockMode): Promise<void> {
    const currentMode = await getCurrentUnlockMode();
    console.log("Current unlock mode in database.ts:", currentMode);
    
    if (currentMode === newMode) {
        throw new Error(`Already in ${newMode} mode`);
    }
    
    return invoke("switch_unlock_mode", { newMode });
}

export async function unlockDatabase(password: string): Promise<void> {
    return invoke("unlock_database", { password });
}

export async function setDatabaseEncryption(newPassword: string, mode: UnlockMode): Promise<void> {
    return invoke("set_database_encryption", {newPassword, mode});
}