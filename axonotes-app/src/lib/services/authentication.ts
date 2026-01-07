import {invoke} from "@tauri-apps/api/core";
import {listen, type UnlistenFn} from "@tauri-apps/api/event";
import type {Profile} from "$lib/types";
import {mode} from "mode-watcher";

export class AuthService {
    /*
     * Returns the active profile, or null if no profile is active.
     */
    static async getActiveProfile(): Promise<Profile | null> {
        return invoke("get_active_profile");
    }

    /*
     * Returns all profiles.
     */
    static async getAllProfiles(): Promise<Profile[]> {
        return invoke<Profile[]>("get_all_profiles");
    }

    /*
     * Starts login flow
     * Returns unlisten functions for cleanup
     */
    static async startLogin(callbacks: {
        onSuccess: (profile: Profile) => void;
        onError: (error: string) => void;
    }) : Promise<UnlistenFn[]> {
        const unlistenSuccess = await listen<Profile>("login-success", (event) => {
            callbacks.onSuccess(event.payload);
        });
        const unlistenError = await listen<string>("login-error", (event) => {
            callbacks.onError(event.payload);
        });
        const authUrl = await invoke<string>("start_login", {darkMode: mode.current === 'dark'});

        await open(authUrl);

        return [unlistenSuccess, unlistenError];
    }
}