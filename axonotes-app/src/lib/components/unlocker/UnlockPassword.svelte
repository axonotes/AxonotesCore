<script lang="ts">
    import { Input } from "$lib/components/ui/input";
    import { Label } from "$lib/components/ui/label";
    import { Button } from "$lib/components/ui/button";
    import { KeyRound } from "@lucide/svelte";
    import {DatabaseService} from "$lib/services/database";
    import * as m from "$lib/paraglide/messages.js";
    import type {UnlockMode} from "$lib/types";
    import {databaseMode, databaseUnlocked} from "$lib/stores/app";

    let { changeUnlockMode } = $props<{
        changeUnlockMode: (mode: UnlockMode) => void;
    }>();

    let password = $state("");
    let isLoading = $state(false);
    let error = $state("");

    async function handleUnlock(e: Event) {
        e.preventDefault();

        if (!password) {
            error = m.auth_unlock_error_incorrect();
            return;
        }

        error = "";
        isLoading = true;

        try {
            await DatabaseService.unlockDatabase(password);
            // Success - navigation will be handled by parent/router
            console.log("Unlocking successful");
            databaseUnlocked.set(true);
            databaseMode.set("pass");
            console.debug(`Database status: ${databaseUnlocked}, database mode: ${databaseMode}`)
        } catch (err) {
            console.error("Unlock failed:", err);
            error = m.auth_unlock_error_incorrect();
            password = ""; // Clear password on error
        } finally {
            isLoading = false;
        }
    }

    async function handleSwitchToPin() {
        try {
            await DatabaseService.switchUnlockMode("pin");
            changeUnlockMode("pin");
        } catch (err) {
            console.error("Failed to switch unlock mode:", err);
            error =
                err instanceof Error ? err.message : m.auth_unlock_error_network();
        }
    }
</script>

<div class="flex h-full w-full items-center justify-center">
    <div class="w-full max-w-sm space-y-8 px-6">
        <!-- Header -->
        <div class="space-y-4 text-center">
            <div
                    class="bg-primary/10 mx-auto flex h-14 w-14 items-center justify-center rounded-full"
            >
                <KeyRound class="text-primary h-7 w-7" />
            </div>
            <div class="space-y-2">
                <h1 class="text-3xl font-semibold leading-tight tracking-tight">
                    {m.auth_unlock_title()}
                </h1>
                <p class="text-muted-foreground text-sm">
                    {m.auth_unlock_description()}
                </p>
            </div>
        </div>

        <!-- Form -->
        <form onsubmit={handleUnlock} class="space-y-5">
            <div class="space-y-2">
                <Label for="password">{m.auth_unlock_password_label()}</Label>
                <Input
                        id="password"
                        type="password"
                        bind:value={password}
                        placeholder={m.auth_unlock_password_placeholder()}
                        disabled={isLoading}
                        class="h-11"
                        aria-invalid={error ? "true" : "false"}
                        aria-describedby={error ? "password-error" : undefined}
                        autofocus
                />
                {#if error}
                    <p id="password-error" class="text-destructive text-sm">
                        {error}
                    </p>
                {/if}
            </div>

            <Button type="submit" class="h-11 w-full" disabled={isLoading}>
                {isLoading
                    ? m.auth_unlock_button_loading()
                    : m.auth_unlock_button_primary()}
            </Button>
        </form>

        <!-- Switch to PIN link -->
        <div class="text-center">
            <button
                    type="button"
                    onclick={handleSwitchToPin}
                    disabled={isLoading}
                    class="text-primary hover:text-primary/90 text-sm underline underline-offset-4 transition-colors disabled:cursor-not-allowed disabled:opacity-50"
            >
                {m.auth_unlock_link_switch_pin()}
            </button>
        </div>
    </div>
</div>