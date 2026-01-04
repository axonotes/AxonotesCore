<script lang="ts">
    import { Label } from "$lib/components/ui/label/index.js";
    import { Button } from "$lib/components/ui/button";
    import { Hash } from "@lucide/svelte";
    import {switchUnlockMode, unlockDatabase, type UnlockMode} from "$lib/services/database";
    import * as m from "$lib/paraglide/messages.js";
    import {goto} from "$app/navigation";

    let { changeUnlockMode } = $props<{
        changeUnlockMode: (mode: UnlockMode) => void;
    }>();

    let pinDigits = $state<string[]>(Array(8).fill(""));
    let pinInputs: HTMLInputElement[] = [];
    let isLoading = $state(false);
    let error = $state("");

    function isValidPinChar(char: string): boolean {
        return /^[0-9A-Z]$/.test(char);
    }

    function handlePinInput(index: number, e: Event) {
        const input = e.target as HTMLInputElement;
        let value = input.value.toUpperCase();

        // If there's already a character and user types a new one, take the last character
        if (value.length > 1) {
            value = value[value.length - 1];
        }

        // Validate character
        if (value && !isValidPinChar(value)) {
            input.value = pinDigits[index];
            return;
        }

        input.value = value;
        pinDigits[index] = value;

        // Autofocus next input if character entered
        if (value && index < 7) {
            pinInputs[index + 1]?.focus();
        }
    }

    function handleKeydown(index: number, e: KeyboardEvent) {
        const input = pinInputs[index];

        // If input has a value and user types a valid character, clear it first to allow overwrite
        if (input.value && e.key.length === 1 && isValidPinChar(e.key.toUpperCase())) {
            input.value = "";
            pinDigits[index] = "";
        }
        // Backspace: clear current and move to previous
        else if (e.key === "Backspace" && !pinDigits[index] && index > 0) {
            pinInputs[index - 1]?.focus();
        }
        // Arrow keys for navigation
        else if (e.key === "ArrowLeft" && index > 0) {
            e.preventDefault();
            pinInputs[index - 1]?.focus();
        } else if (e.key === "ArrowRight" && index < 7) {
            e.preventDefault();
            pinInputs[index + 1]?.focus();
        }
    }

    function handlePaste(e: ClipboardEvent) {
        e.preventDefault();
        const pastedData = e.clipboardData?.getData("text");
        if (!pastedData) return;

        // Extract only valid alphanumeric characters (0-9, A-Z)
        const chars = pastedData
            .toUpperCase()
            .split("")
            .filter((char) => isValidPinChar(char))
            .slice(0, 8);

        chars.forEach((char, index) => {
            pinDigits[index] = char;
            if (pinInputs[index]) {
                pinInputs[index].value = char;
            }
        });

        // Focus the next empty input or last input
        const nextEmptyIndex = chars.length < 8 ? chars.length : 7;
        pinInputs[nextEmptyIndex]?.focus();
    }

    async function handleUnlock(e: Event) {
        e.preventDefault();

        const pin = pinDigits.join("");

        if (pin.length !== 8 || !pin.split("").every(isValidPinChar)) {
            error = m.auth_unlock_pin_error_invalid();
            return;
        }

        error = "";
        isLoading = true;

        try {
            console.log("pin:", pin);
            await unlockDatabase(pin);
            // Success - navigation will be handled by parent/router
            console.log("Unlocking successful");
            await goto("/app");
        } catch (err) {
            console.error("Unlock failed:", err);
            error = m.auth_unlock_pin_error_incorrect();
            // Clear PIN on error
            pinDigits = Array(8).fill("");
            pinInputs.forEach((input) => (input.value = ""));
            pinInputs[0]?.focus();
        } finally {
            isLoading = false;
        }
    }

    async function handleSwitchToPassword() {
        try {
            await switchUnlockMode("pass");
            changeUnlockMode("pass");
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
                <Hash class="text-primary h-7 w-7" />
            </div>
            <div class="space-y-2">
                <h1 class="text-3xl font-semibold leading-tight tracking-tight">
                    {m.auth_unlock_pin_title()}
                </h1>
                <p class="text-muted-foreground text-sm">
                    {m.auth_unlock_pin_description()}
                </p>
            </div>
        </div>

        <!-- Form -->
        <form onsubmit={handleUnlock} class="space-y-5">
            <div class="space-y-2">
                <Label for="pin-0">{m.auth_unlock_pin_label()}</Label>
                <div class="flex gap-2">
                    {#each Array(8) as _, i}
                        <input
                                id={i === 0 ? "pin-0" : undefined}
                                bind:this={pinInputs[i]}
                                type="text"
                                pattern="[0-9A-Z]"
                                maxlength="1"
                                class="border-input bg-background ring-offset-background placeholder:text-muted-foreground focus-visible:ring-ring flex h-11 w-11 rounded-md border text-center text-sm uppercase transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                                disabled={isLoading}
                                oninput={(e) => handlePinInput(i, e)}
                                onkeydown={(e) => handleKeydown(i, e)}
                                onpaste={i === 0 ? handlePaste : undefined}
                                aria-label={`PIN character ${i + 1}`}
                                aria-invalid={error ? "true" : "false"}
                                aria-describedby={error && i === 0 ? "pin-error" : undefined}
                                autofocus={i === 0}
                        />
                        <!-- keep autofocus -->
                    {/each}
                </div>
                {#if error}
                    <p id="pin-error" class="text-destructive text-sm">
                        {error}
                    </p>
                {/if}
            </div>

            <Button type="submit" class="h-11 w-full" disabled={isLoading}>
                {isLoading
                    ? m.auth_unlock_pin_button_loading()
                    : m.auth_unlock_pin_button_primary()}
            </Button>
        </form>

        <!-- Switch to password link -->
        <div class="text-center">
            <button
                    type="button"
                    onclick={handleSwitchToPassword}
                    disabled={isLoading}
                    class="text-primary hover:text-primary/90 text-sm underline underline-offset-4 transition-colors disabled:cursor-not-allowed disabled:opacity-50"
            >
                {m.auth_unlock_pin_link_switch_password()}
            </button>
        </div>
    </div>
</div>