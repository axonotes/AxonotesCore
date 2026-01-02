import {invoke} from "@tauri-apps/api/core";
import {listen, type UnlistenFn} from "@tauri-apps/api/event";
import {open} from "@tauri-apps/plugin-shell";
import {mode} from "mode-watcher";

export interface Profile {
  id: string;
  email: string;
  name: string;
}

/**
 * Auth Service
 * Handles all authentication-related operations
 */
export class AuthService {
  /**
   * Get the currently active profile
   */
  static async getActiveProfile(): Promise<Profile | null> {
    return await invoke<Profile | null>("get_active_profile");
  }

  /**
   * Get all profiles
   */
  static async getAllProfiles(): Promise<Profile[]> {
    return await invoke<Profile[]>("get_all_profiles");
  }

  /**
   * Start login flow
   * Returns unlisten functions for cleanup
   */
  static async startLogin(callbacks: {
    onSuccess: (profile: Profile) => void;
    onError: (error: string) => void;
  }): Promise<UnlistenFn[]> {
    // Setup event listeners
    const unlistenSuccess = await listen<Profile>("login-success", (event) => {
      callbacks.onSuccess(event.payload);
    });

    const unlistenError = await listen<string>("login-error", (event) => {
      callbacks.onError(event.payload);
    });

    // Get current theme
    let darkMode = mode.current === "dark";

    // Get auth URL and open browser
    const authUrl = await invoke<string>("start_login", {
      darkMode,
    });
    await open(authUrl);

    return [unlistenSuccess, unlistenError];
  }

  /**
   * Switch active profile
   */
  static async switchProfile(profileId: string): Promise<void> {
    await invoke("switch_profile", {profileId});
  }

  /**
   * Logout (delete profile)
   */
  static async logout(profileId: string): Promise<void> {
    await invoke("logout", {profileId});
  }

  /**
   * Refresh access token for active profile
   */
  static async refreshToken(): Promise<string> {
    return await invoke<string>("refresh_token");
  }
}
