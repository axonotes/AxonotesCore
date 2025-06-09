<script lang="ts">
    import {goto} from "$app/navigation";
    import {unlockVaultWithPassword} from "$lib/client/cryptoStore";
    import {base64ToUint8Array} from "$lib/client/crypto";
    import {onMount} from "svelte";
    import {
        connectToSpacetime,
        ensureSpacetimeConnected,
        getSpacetimeUser,
        spacetime,
    } from "$lib/client/spacetime";

    let password = $state("");
    let isLoading = $state(false);
    let errorMessage = $state("");

    async function handleUnlock() {
        if (!password) {
            errorMessage = "Please enter your master password.";
            return;
        }
        isLoading = true;
        errorMessage = "";

        const currentUser = await getSpacetimeUser();
        if (!currentUser) {
            console.error("Error getting spacetime user");
            return;
        }
        if (
            !currentUser?.publicKey ||
            !currentUser.encryptedPrivateKey ||
            !currentUser.argonSalt
        ) {
            await goto("/auth/setup");
            return null;
        }

        try {
            // The salt is a Base64 string from the server; convert it to a Uint8Array.
            const saltBytes = base64ToUint8Array(currentUser.argonSalt);

            const success = await unlockVaultWithPassword(
                JSON.parse(currentUser.encryptedPrivateKey),
                password,
                saltBytes
            );

            if (success) {
                // On successful unlock, the cryptoStore is now populated.
                // Navigate the user to their main dashboard.
                await goto("/dashboard");
            } else {
                errorMessage = "Invalid password. Please try again.";
            }
        } catch (err) {
            console.error("An unexpected error occurred during unlock:", err);
            errorMessage = "An unexpected error occurred. Please try again.";
        } finally {
            // Clear password field for security, regardless of outcome.
            password = "";
            isLoading = false;
        }
    }

    onMount(() => {
        connectToSpacetime();
    });
</script>

<div class="grid w-full items-center lg:mt-24">
    <div
        class="card outline-primary-500 dark:shadow-primary-900 shadow-primary-400 m-auto w-full max-w-md p-4 shadow-2xl outline-1 md:p-8"
    >
        <div class="flex flex-col space-y-6">
            <h1 class="h2 text-center">Unlock Your Vault</h1>
            <div>
                <p class="mb-8 text-center">
                    Enter your master password to decrypt your data.
                </p>
                <form
                    on:submit|preventDefault={handleUnlock}
                    class="flex flex-col"
                >
                    <label for="password" class="mb-2">Master Password</label>
                    <input
                        id="password"
                        type="password"
                        bind:value={password}
                        placeholder="••••••••••••"
                        class="input mb-1"
                        required
                    />

                    <button
                        type="submit"
                        disabled={isLoading}
                        class="btn preset-filled-primary-500 mt-8"
                    >
                        {#if isLoading}
                            <span class="animate-pulse">Unlocking...</span>
                        {:else}
                            Unlock
                        {/if}
                    </button>
                </form>

                <div class="mt-4 text-center">
                    <a href="/auth/recover" class="anchor text-sm">
                        Forgot your password?
                    </a>
                </div>
            </div>
        </div>

        {#if errorMessage}
            <p class="text-error-500 mt-4 text-center">{errorMessage}</p>
        {/if}
    </div>
</div>
