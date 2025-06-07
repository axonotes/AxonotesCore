<!-- src/routes/auth/setup/+page.svelte -->
<script lang="ts">
    import {generateMnemonic} from "bip39";
    import zxcvbn from "zxcvbn";
    import * as crypto from "$lib/client/crypto";
    import {enhance} from "$app/forms";

    let masterPassword = $state("");
    let isLoading = $state(false);
    let errorMessage = $state("");
    let generatedPassphrase = $state("");

    let passwordStrength = $derived(
        masterPassword ? zxcvbn(masterPassword).score : 0,
    );

    async function handleSetup(event: SubmitEvent) {
        event.preventDefault();
        if (passwordStrength < 3) {
            errorMessage = "Password is too weak. Please choose a stronger one.";
            return;
        }
        isLoading = true;
        errorMessage = "";

        try {
            const {publicKey, privateKey} = await crypto.generateRsaKeyPair();
            const backupPassphrase = generateMnemonic(128);
            generatedPassphrase = backupPassphrase;

            const salt = crypto.generateSalt(16);
            const passwordHash = await crypto.hashPassword(masterPassword, salt);

            const encryptedPrivateKey = await crypto.encryptWithAes(
                privateKey,
                passwordHash,
            );

            const backupKey = crypto.getKeyFromMnemonic(backupPassphrase);
            const encryptedBackupKey = await crypto.encryptWithAes(
                privateKey,
                backupKey,
            );

            const payload = {
                publicKey,
                encryptedPrivateKey: JSON.stringify(encryptedPrivateKey),
                encryptedBackupKey: JSON.stringify(encryptedBackupKey),
                argonSalt: crypto.arrayBufferToBase64(salt.buffer),
            };

            const response = await fetch("/api/user/setup-keys", {
                method: "POST",
                headers: {"Content-Type": "application/json"},
                body: JSON.stringify(payload),
            });

            if (!response.ok) {
                throw new Error("Failed to save keys to the server.");
            }
        } catch (err) {
            errorMessage = "An unexpected error occurred. Please try again.";
            console.error(err);
        } finally {
            masterPassword = "";
            isLoading = false;
        }
    }
</script>

<h1>Set Up Your Secure Account</h1>

{#if !generatedPassphrase}
    <p>Choose a strong master password. This will be used to encrypt your data.</p>
    <form onsubmit={handleSetup} use:enhance>
        <input
                type="password"
                bind:value={masterPassword}
                placeholder="Master Password"
        />
        <progress value={passwordStrength} max="4"></progress>
        <button type="submit" disabled={isLoading || passwordStrength < 3}>
            {isLoading ? "Generating..." : "Create Account"}
        </button>
    </form>
{:else}
    <h2>IMPORTANT: Save Your Backup Passphrase!</h2>
    <p>
        If you forget your master password, this is the ONLY way to recover your
        account. Store it somewhere safe and offline.
    </p>
    <pre>{generatedPassphrase}</pre>
    <a href="/dashboard">I have saved my passphrase. Continue to Dashboard.</a>
{/if}

{#if errorMessage}
    <p style="color: red;">{errorMessage}</p>
{/if}