import {writable, derived} from "svelte/store";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import {open} from "@tauri-apps/plugin-shell";

export interface Profile {
  id: string;
  email: string;
  name: string;
  access_token: string;
  refresh_token?: string;
}

export interface ProfilesState {
  profiles: Profile[];
  active_profile_id: string | null;
}

// Store for profiles state
export const profilesState = writable<ProfilesState>({
  profiles: [],
  active_profile_id: null,
});

// Derived store for active profile
export const activeProfile = derived(
  profilesState,
  ($state) =>
    $state.profiles.find((p) => p.id === $state.active_profile_id) || null
);

// Loading state
export const isLoading = writable(false);

// Initialize auth store
export async function initAuth() {
  try {
    const state = await invoke<ProfilesState>("get_profiles");
    profilesState.set(state);
  } catch (error) {
    console.error("Failed to load profiles:", error);
  }
}

// Start OAuth flow
export async function startLogin() {
  isLoading.set(true);

  try {
    // Setup listener for OAuth callback
    const unlisten = await listen<string>("oauth_callback", async (event) => {
      try {
        const code = event.payload;

        // Exchange code for token
        const profile = await invoke<Profile>("exchange_code_for_token", {
          code,
        });

        // Add profile to store
        const newState = await invoke<ProfilesState>("add_profile", {profile});
        profilesState.set(newState);

        isLoading.set(false);
        unlisten();
      } catch (error) {
        console.error("Failed to exchange code:", error);
        isLoading.set(false);
        unlisten();
      }
    });

    // Setup error listener
    const unlistenError = await listen<string>("oauth_error", (event) => {
      console.error("OAuth error:", event.payload);
      isLoading.set(false);
      unlistenError();
    });

    // Get authorization URL
    const authUrl = await invoke<string>("start_oauth_flow");

    // Open in system browser
    await open(authUrl);
  } catch (error) {
    console.error("Failed to start OAuth flow:", error);
    isLoading.set(false);
  }
}

// Switch active profile
export async function switchProfile(profileId: string) {
  try {
    const newState = await invoke<ProfilesState>("set_active_profile", {
      profileId,
    });
    profilesState.set(newState);
  } catch (error) {
    console.error("Failed to switch profile:", error);
  }
}

// Logout (remove profile)
export async function logout(profileId: string) {
  try {
    const newState = await invoke<ProfilesState>("remove_profile", {
      profileId,
    });
    profilesState.set(newState);

    if (newState.profiles.length === 0) {
      window.location.href = "/login";
    }
  } catch (error) {
    console.error("Failed to logout:", error);
  }
}

// Refresh access token
export async function refreshToken(profileId: string) {
  try {
    const newToken = await invoke<string>("refresh_access_token", {
      profileId,
    });

    await invoke("update_profile_token", {
      profileId,
      accessToken: newToken,
    });

    // Reload profiles
    const state = await invoke<ProfilesState>("get_profiles");
    profilesState.set(state);

    return newToken;
  } catch (error) {
    console.error("Failed to refresh token:", error);
    throw error;
  }
}
