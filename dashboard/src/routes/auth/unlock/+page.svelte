<script lang="ts">
    import {goto} from "$app/navigation";
    import {unlockVaultWithPassword} from "$lib/client/cryptoStore";
    import {base64ToUint8Array} from "$lib/client/crypto";
    import {onMount} from "svelte";
    import {connectToSpacetime, getSpacetimeUser} from "$lib/client/spacetime";

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
            errorMessage =
                "Could not load user data. Please check your connection.";
            isLoading = false;
            return;
        }

        if (
            !currentUser.encryptedPrivateKey ||
            !currentUser.encryptedPrivateSigningKey ||
            !currentUser.argonSalt
        ) {
            await goto("/auth/setup");
            return;
        }

        try {
            const saltBytes = base64ToUint8Array(currentUser.argonSalt);

            // Call the updated unlock function with both encrypted keys.
            const success = await unlockVaultWithPassword(
                JSON.parse(currentUser.encryptedPrivateKey),
                JSON.parse(currentUser.encryptedPrivateSigningKey),
                password,
                saltBytes
            );

            if (success) {
                // On successful unlock, the vaultStore is now populated with both keys.
                await goto("/dashboard");
            } else {
                errorMessage = "Invalid password. Please try again.";
            }
        } catch (err) {
            console.error("An unexpected error occurred during unlock:", err);
            // The most common error here is a bad password causing a decrypt failure.
            errorMessage = "Invalid password. Please try again.";
        } finally {
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
                <form onsubmit={handleUnlock} class="flex flex-col">
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
