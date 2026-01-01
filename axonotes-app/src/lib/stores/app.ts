import {writable, derived, get} from "svelte/store";
import {DatabaseService, type UnlockMode} from "$lib/services/database";
import {AuthService, type Profile} from "$lib/services/auth";

/**
 * Application State
 * Single source of truth for app-level state
 */

// Database state
export const databaseUnlocked = writable(false);
export const databaseMode = writable<UnlockMode>("none");

// Auth state
export const activeProfile = writable<Profile | null>(null);
export const allProfiles = writable<Profile[]>([]);

// UI state
export const isInitialized = writable(false);
export const isLoading = writable(false);

/**
 * Derived state
 */
export const isAuthenticated = derived(activeProfile, ($profile) => $profile !== null);

export const needsUnlock = derived(
  [databaseUnlocked, databaseMode],
  ([$unlocked, $mode]) => !$unlocked && $mode !== "none"
);

/**
 * App Actions
 * Business logic orchestration
 */
export const app = {
  /**
   * Initialize the application
   * Call this once on app start
   */
  async initialize() {
    console.log("[App] Initializing...");

    try {
      // Step 1: Get unlock mode (reads config file, doesn't need DB)
      const mode = await DatabaseService.getMode();
      databaseMode.set(mode);
      console.log(`[App] Unlock mode: ${mode}`);

      // Step 2: Check if DB is already unlocked (checks global var, doesn't need DB)
      const unlocked = await DatabaseService.isUnlocked();
      databaseUnlocked.set(unlocked);
      console.log(`[App] Database unlocked: ${unlocked}`);

      // Step 3: If not unlocked and mode is "none", try auto-unlock
      if (!unlocked && mode === "none") {
        console.log("[App] Auto-unlocking database (mode: none)");
        try {
          await DatabaseService.unlock("");
          databaseUnlocked.set(true);
          console.log("[App] Auto-unlock successful");
        } catch (unlockError) {
          // Auto-unlock failed - DB might be encrypted but config says "none"
          // This is a state desync - don't crash, just leave it locked
          console.error("[App] Auto-unlock failed:", unlockError);
          console.warn("[App] Database may be encrypted. Config says mode='none' but DB file might be encrypted.");
          console.warn("[App] User will need to manually unlock or wipe the database.");
          databaseUnlocked.set(false);
          // Don't throw - let initialization continue
        }
      }

      // Step 4: Load profiles ONLY if database is unlocked
      if (get(databaseUnlocked)) {
        await this.loadAuth();
      } else {
        console.log("[App] Database locked, skipping profile load");
      }

      isInitialized.set(true);
      console.log("[App] Initialization complete");
    } catch (error) {
      console.error("[App] Initialization failed:", error);
      // Set initialized anyway so UI can render (will show unlock/login pages)
      isInitialized.set(true);
      // Don't throw - let the UI handle it
    }
  },

  /**
   * Load authentication data
   * REQUIRES database to be unlocked
   */
  async loadAuth() {
    console.log("[App] Loading auth data...");
    try {
      const [active, all] = await Promise.all([
        AuthService.getActiveProfile(),
        AuthService.getAllProfiles(),
      ]);

      activeProfile.set(active);
      allProfiles.set(all);

      console.log(`[App] Loaded auth: active=${active?.email}, total=${all.length}`);
    } catch (error) {
      console.error("[App] Failed to load auth:", error);
      // Don't throw - this might happen if DB is empty
      activeProfile.set(null);
      allProfiles.set([]);
    }
  },

  /**
   * Unlock database with password
   */
  async unlockDatabase(password: string) {
    try {
      isLoading.set(true);
      console.log("[App] Unlocking database...");

      await DatabaseService.unlock(password);
      databaseUnlocked.set(true);
      console.log("[App] Database unlocked");

      // Load auth after successful unlock
      await this.loadAuth();

      console.log("[App] Unlock complete");
    } catch (error) {
      console.error("[App] Failed to unlock database:", error);
      databaseUnlocked.set(false);
      throw error;
    } finally {
      isLoading.set(false);
    }
  },

  /**
   * Start login flow
   */
  async startLogin() {
    isLoading.set(true);

    try {
      const unlisteners = await AuthService.startLogin({
        onSuccess: async (profile) => {
          console.log("[App] Login successful:", profile.email);
          await this.loadAuth();
          isLoading.set(false);

          // Clean up listeners
          unlisteners.forEach((fn) => fn());
        },
        onError: (error) => {
          console.error("[App] Login failed:", error);
          isLoading.set(false);

          // Clean up listeners
          unlisteners.forEach((fn) => fn());
        },
      });
    } catch (error) {
      console.error("[App] Failed to start login:", error);
      isLoading.set(false);
      throw error;
    }
  },

  /**
   * Logout current profile
   */
  async logout(profileId: string) {
    try {
      await AuthService.logout(profileId);
      await this.loadAuth();
      console.log("[App] Logged out");
    } catch (error) {
      console.error("[App] Logout failed:", error);
      throw error;
    }
  },

  /**
   * Switch active profile
   */
  async switchProfile(profileId: string) {
    try {
      await AuthService.switchProfile(profileId);
      await this.loadAuth();
      console.log("[App] Switched profile");
    } catch (error) {
      console.error("[App] Failed to switch profile:", error);
      throw error;
    }
  },
};
