import {derived, get, writable } from 'svelte/store';
import type { Profile, UnlockMode } from '$lib/types';
import {AuthService} from "$lib/services/authentication";
import {DatabaseService} from "$lib/services/database";

export const isLoading = writable(false);
export const isInitialized = writable(false);
export const databaseUnlocked = writable(false);
export const databaseMode = writable<UnlockMode | null>("none");
export const activeProfile = writable<Profile | null>(null);
export const allProfiles = writable<Profile[]>([]);

export const isAuthenticated = derived(
    activeProfile,
    ($profile) => $profile !== null
);
export const isUnlocked = derived(
    [databaseUnlocked, databaseMode],
    ([$unlocked, $mode]) => $unlocked || $mode === "none"
);

export const app = {
    /**
     * Initialize the app. Needs to be called at app start.
     */
    async initialize() {
        if (get(isInitialized)) {
            console.log("[App] Already initialized. Skipping...");
            return;
        }

        console.log("[App] Initializing...");

        try {
            const mode = await DatabaseService.getCurrentUnlockMode();
            databaseMode.set(mode);
            console.log(`[App] Unlock mode: ${mode}`);

            const unlocked = await DatabaseService.isDatabaseUnlocked();
            databaseUnlocked.set(unlocked);
            console.log(`[App] Database unlocked: ${unlocked}`);

            if (!unlocked && mode === "none") {
                console.log("[App] Auto-unlocking the database (mode: none)...");
                try {
                    await DatabaseService.unlockDatabase("");
                    databaseUnlocked.set(true);
                    console.log("[App] Auto-unlock successful");
                } catch (unlockError) {
                    console.error("[App] Auto-unlock failed:", unlockError);
                    console.warn("[App] Database may be encrypted, while config state has mode='none'.");
                    console.warn("[App] User will need to manually unlock or wipe the database");
                    databaseUnlocked.set(false);
                }
            }

            if (get(databaseUnlocked)) {
                await this.loadAuthData();
            } else {
                console.log("[App] Database is locked, skipping loading profiles");
            }

            isInitialized.set(true);
            console.log("[App] Initialized");
        } catch (error) {
            console.error("[App] Failed to initialize:", error);
            isInitialized.set(false);
        }
    },

    /**
     * Loads authentication data (after unlocking db)
     */
    async loadAuthData() {
        console.log("[App] Loading authentication data...");

        try {
            const [active, all] = await Promise.all([
                AuthService.getActiveProfile(),
                AuthService.getAllProfiles(),
            ]);

            activeProfile.set(active);
            allProfiles.set(all);

            console.log(`[App] Loaded authentication data: active=${active?.email}, total=${all.length}`);
        } catch (error) {
            console.error("[App] Failed to load authentication data:", error);
            activeProfile.set(null);
            allProfiles.set([]);
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
                    console.log("[App] Login successful for user:", profile.email);
                    await this.loadAuthData();
                    isLoading.set(false);

                    // Clean up listeners
                    unlisteners.forEach((fn) => fn());
                },
                onError: (error) => {
                    console.error("[App] Login failed:", error);
                    isLoading.set(false);

                    // Clean up listeners
                    unlisteners.forEach((fn) => fn());
                }
            });
        } catch (error) {
            console.error("[App] Failed to start login process:", error);
            isLoading.set(false);
            throw error;
        }
    }
}