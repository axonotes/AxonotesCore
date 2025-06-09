<script lang="ts">
    import {goto} from "$app/navigation";
    import zxcvbn from "zxcvbn";
    import * as crypto from "$lib/client/crypto";
    import {unlockVaultWithPassword} from "$lib/client/cryptoStore";
    import {Progress} from "@skeletonlabs/skeleton-svelte";
    import {
        connectToSpacetime,
        ensureSpacetimeConnected,
        getSpacetimeUser,
    } from "$lib/client/spacetime";
    import {onMount} from "svelte";

    let recoveryStep = $state(1);
    let isLoading = $state(false);
    let errorMessage = $state("");

    // Step 1 state
    let passphrase = $state("");
    let recoveredRsaPrivateKey = $state("");
    let recoveredEd25519PrivateKey = $state("");

    // Step 2 state
    let newPassword = $state("");
    let confirmPassword = $state("");

    const zxcvbnResult = $derived(newPassword ? zxcvbn(newPassword) : null);
    const passwordStrength = $derived(zxcvbnResult ? zxcvbnResult.score : 0);

    /**
     * Step 1: User submits their backup passphrase. We use it to decrypt BOTH
     * the main private key and the private signing key from their respective backups.
     */
    async function handlePassphraseSubmit() {
        isLoading = true;
        errorMessage = "";

        const currentUser = await getSpacetimeUser();
        if (
            !currentUser?.encryptedBackupKey ||
            !currentUser.encryptedPrivateBackupSigningKey
        ) {
            errorMessage = "Account backup data is missing. Cannot recover.";
            isLoading = false;
            return;
        }

        try {
            // Derive the single backup key from the user's mnemonic
            const backupKey = crypto.getKeyFromMnemonic(passphrase);

            // Decrypt the RSA private key using the backup key
            recoveredRsaPrivateKey = await crypto.decryptWithAes(
                JSON.parse(currentUser.encryptedBackupKey),
                backupKey
            );

            // Decrypt the Ed25519 private key using the backup key
            recoveredEd25519PrivateKey = await crypto.decryptWithAes(
                JSON.parse(currentUser.encryptedPrivateBackupSigningKey),
                backupKey
            );

            recoveryStep = 2;
        } catch (err) {
            console.error("Failed to decrypt with passphrase", err);
            errorMessage =
                "Invalid passphrase. Please check your words and try again.";
        } finally {
            isLoading = false;
        }
    }

    /**
     * Step 2: User sets a new master password. This logic is now correct because
     * we successfully recovered the signing key in Step 1.
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
                recoveredRsaPrivateKey,
                newPasswordHash
            );
            const newEncryptedPrivateSigningKey = await crypto.encryptWithAes(
                recoveredEd25519PrivateKey,
                newPasswordHash
            );

            const newEncryptedPrivateKeyJson = JSON.stringify(
                newEncryptedPrivateKey
            );
            const newEncryptedPrivateSigningKeyJson = JSON.stringify(
                newEncryptedPrivateSigningKey
            );
            const newSaltBase64 = crypto.arrayBufferToBase64(newSalt.buffer);

            const messageToSign = `${newEncryptedPrivateKeyJson}${newEncryptedPrivateSigningKeyJson}${newSaltBase64}`;

            const signingKeyBytes = crypto.base64ToUint8Array(
                recoveredEd25519PrivateKey
            );
            const signingKeyHandle =
                await crypto.importEd25519PrivateKey(signingKeyBytes);
            const signature = await crypto.signData(
                signingKeyHandle,
                messageToSign
            );

            console.log(messageToSign);

            const handle = await ensureSpacetimeConnected();
            handle.connection!.reducers.updateEncryptionKeys(
                newEncryptedPrivateKeyJson,
                newEncryptedPrivateSigningKeyJson,
                newSaltBase64,
                signature
            );

            const success = await unlockVaultWithPassword(
                newEncryptedPrivateKey,
                newEncryptedPrivateSigningKey,
                newPassword,
                newSalt
            );

            if (success) {
                await goto("/dashboard");
            } else {
                throw new Error("Failed to unlock vault with new password.");
            }
        } catch (err: any) {
            console.error("An error occurred during password reset:", err);
            errorMessage =
                err.message ||
                "An unexpected error occurred. Please try again.";
        } finally {
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
