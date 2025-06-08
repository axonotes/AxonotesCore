<script lang="ts">
    import {goto} from "$app/navigation";
    import zxcvbn from "zxcvbn";
    import * as crypto from "$lib/client/crypto";
    import {unlockVaultWithPassword} from "$lib/client/cryptoStore";
    import {Progress} from "@skeletonlabs/skeleton-svelte";

    let {data} = $props();

    let recoveryStep = $state(1); // 1: Enter passphrase, 2: Set new password
    let isLoading = $state(false);
    let errorMessage = $state("");

    // Step 1 state
    let passphrase = $state("");
    let recoveredPrivateKey = $state(""); // Will hold the raw private key string

    // Step 2 state
    let newPassword = $state("");
    let confirmPassword = $state("");

    const zxcvbnResult = $derived(newPassword ? zxcvbn(newPassword) : null);
    const passwordStrength = $derived(zxcvbnResult ? zxcvbnResult.score : 0);

    /**
     * Step 1: User submits their backup passphrase. We use it to decrypt the
     * backup key and recover the raw private key.
     */
    async function handlePassphraseSubmit() {
        isLoading = true;
        errorMessage = "";
        try {
            const backupKey = crypto.getKeyFromMnemonic(passphrase);
            const privateKeyString = await crypto.decryptWithAes(
                data.encryptedBackupKey,
                backupKey
            );
            recoveredPrivateKey = privateKeyString;
            recoveryStep = 2; // Move to the next step
        } catch (err) {
            console.error("Failed to decrypt with passphrase", err);
            errorMessage =
                "Invalid passphrase. Please check your words and try again.";
        } finally {
            isLoading = false;
        }
    }

    /**
     * Step 2: User sets a new master password. We use it to re-encrypt the
     * recovered private key and update the server.
     */
    async function handleNewPasswordSubmit() {
        if (newPassword !== confirmPassword) {
            errorMessage = "Passwords do not match.";
            return;
        }
        if (passwordStrength < 3) {
            errorMessage = "New password is too weak.";
            return;
        }

        isLoading = true;
        errorMessage = "";

        try {
            const newSalt = crypto.generateSalt(16);
            const newPasswordHash = await crypto.hashPassword(
                newPassword,
                newSalt
            );
            const newEncryptedPrivateKey = await crypto.encryptWithAes(
                recoveredPrivateKey,
                newPasswordHash
            );

            // We reuse the setup-keys endpoint to update the user's record.
            // The public key and encrypted backup key remain unchanged.
            const payload = {
                publicKey: data.publicKey,
                encryptedPrivateKey: JSON.stringify(newEncryptedPrivateKey),
                encryptedBackupKey: JSON.stringify(data.encryptedBackupKey),
                argonSalt: crypto.arrayBufferToBase64(newSalt.buffer),
            };

            const response = await fetch("/api/user/setup-keys", {
                method: "POST",
                headers: {"Content-Type": "application/json"},
                body: JSON.stringify(payload),
            });

            if (!response.ok) {
                throw new Error(
                    "Failed to save new key details to the server."
                );
            }

            // Now that the server is updated, unlock the vault for the current session.
            const success = await unlockVaultWithPassword(
                newEncryptedPrivateKey,
                newPassword,
                newSalt
            );

            if (success) {
                await goto("/dashboard");
            } else {
                // This should theoretically not fail, but handle it just in case.
                throw new Error("Failed to unlock vault with new password.");
            }
        } catch (err) {
            console.error("An error occurred during password reset:", err);
            errorMessage = "An unexpected error occurred. Please try again.";
        } finally {
            isLoading = false;
        }
    }
</script>

<div class="grid w-full items-center lg:mt-24">
    <div
        class="card outline-primary-500 dark:shadow-primary-900 shadow-primary-400 m-auto w-full max-w-md p-4 shadow-2xl outline-1 md:p-8"
    >
        {#if recoveryStep === 1}
            <div class="flex flex-col space-y-6">
                <h1 class="h2 text-center">Recover Account</h1>
                <p class="text-center">
                    Enter your 12-word backup passphrase to begin the recovery
                    process.
                </p>
                <form
                    on:submit|preventDefault={handlePassphraseSubmit}
                    class="flex flex-col"
                >
                    <label for="passphrase" class="mb-2"
                        >Backup Passphrase</label
                    >
                    <textarea
                        id="passphrase"
                        bind:value={passphrase}
                        rows="3"
                        class="textarea p-2"
                        placeholder="word1 word2 word3..."
                        required
                    ></textarea>
                    <button
                        type="submit"
                        disabled={isLoading}
                        class="btn preset-filled-primary-500 mt-8"
                    >
                        {isLoading ? "Verifying..." : "Continue"}
                    </button>
                </form>
            </div>
        {:else if recoveryStep === 2}
            <div class="flex flex-col space-y-6">
                <h1 class="h2 text-center">Set New Password</h1>
                <p class="text-center">
                    Your passphrase was correct. Now, choose a new strong master
                    password.
                </p>
                <form
                    on:submit|preventDefault={handleNewPasswordSubmit}
                    class="flex flex-col"
                >
                    <label for="newPassword" class="mb-2"
                        >New Master Password</label
                    >
                    <input
                        id="newPassword"
                        type="password"
                        bind:value={newPassword}
                        class="input mb-1"
                        required
                    />
                    <Progress value={passwordStrength} max={4}></Progress>

                    <label for="confirmPassword" class="mb-2 mt-4"
                        >Confirm New Password</label
                    >
                    <input
                        id="confirmPassword"
                        type="password"
                        bind:value={confirmPassword}
                        class="input"
                        required
                    />

                    <button
                        type="submit"
                        disabled={isLoading || passwordStrength < 3}
                        class="btn mt-8 {passwordStrength < 3
                            ? 'preset-outlined-primary-500'
                            : 'preset-filled-primary-500'}"
                    >
                        {isLoading ? "Saving..." : "Reset Password and Login"}
                    </button>
                </form>
            </div>
        {/if}

        {#if errorMessage}
            <p class="text-error-500 mt-4 text-center">{errorMessage}</p>
        {/if}
    </div>
</div>
