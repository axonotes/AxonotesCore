<script lang="ts">
    import {generateMnemonic} from "bip39";
    import zxcvbn from "zxcvbn";
    import * as crypto from "$lib/client/crypto";
    import {enhance} from "$app/forms";
    import * as persistentKeyStore from "$lib/client/persistentKeyStore";
    import {Progress} from "@skeletonlabs/skeleton-svelte";
    import {copy} from "$lib/actions/copy";
    import {TriangleAlert} from "@lucide/svelte";
    import {onMount} from "svelte";
    import {
        connectToSpacetime,
        ensureSpacetimeConnected,
        spacetime,
    } from "$lib/client/spacetime";

    let masterPassword = $state("");
    let isLoading = $state(false);
    let errorMessage = $state("");
    let generatedPassphrase = $state("");

    let confirmedSaved = $state(false);

    const zxcvbnResult = $derived(
        masterPassword ? zxcvbn(masterPassword) : null
    );

    const passwordStrength = $derived(zxcvbnResult ? zxcvbnResult.score : 0);

    const passwordStrengthFeedback = $derived(
        zxcvbnResult ? zxcvbnResult.feedback : null
    );

    type SetEncryptionPayload = {
        publicKey: string;
        encryptedPrivateKey: string;
        encryptedBackupKey: string;
        argonSalt: string;
    };

    async function handleSetup(event: SubmitEvent) {
        event.preventDefault();
        if (passwordStrength < 3) {
            errorMessage =
                "Password is too weak. Please choose a stronger one.";
            return;
        }
        isLoading = true;
        errorMessage = "";

        try {
            const {publicKey, privateKey} = await crypto.generateRsaKeyPair();
            const backupPassphrase = generateMnemonic(128);
            generatedPassphrase = backupPassphrase;

            const salt = crypto.generateSalt(16);
            const passwordHash = await crypto.hashPassword(
                masterPassword,
                salt
            );

            const encryptedPrivateKey = await crypto.encryptWithAes(
                privateKey,
                passwordHash
            );

            const backupKey = crypto.getKeyFromMnemonic(backupPassphrase);
            const encryptedBackupKey = await crypto.encryptWithAes(
                privateKey,
                backupKey
            );

            const payload: SetEncryptionPayload = {
                publicKey,
                encryptedPrivateKey: JSON.stringify(encryptedPrivateKey),
                encryptedBackupKey: JSON.stringify(encryptedBackupKey),
                argonSalt: crypto.arrayBufferToBase64(salt.buffer),
            };

            await callSetEncryptReducer(payload);

            const privateKeyBytes = crypto.base64ToUint8Array(privateKey);
            const nonExtractableKey =
                await crypto.importPrivateKey(privateKeyBytes);
            await persistentKeyStore.storeKey(nonExtractableKey);
        } catch (err) {
            errorMessage = "An unexpected error occurred. Please try again.";
            console.error(err);
        } finally {
            masterPassword = "";
            isLoading = false;
        }
    }

    async function callSetEncryptReducer(payload: SetEncryptionPayload) {
        const handle = await ensureSpacetimeConnected();

        if (!handle.connection) {
            console.error("Error, no handle was provided");
            return;
        }

        handle.connection.reducers.setEncryption(
            payload.publicKey,
            payload.encryptedPrivateKey,
            payload.encryptedBackupKey,
            payload.argonSalt
        );
    }

    onMount(() => {
        connectToSpacetime();
    });
</script>

<div class="grid w-full items-center lg:mt-24">
    <div
        class="card outline-primary-500 dark:shadow-primary-900 shadow-primary-400 m-auto w-full max-w-xl p-4 shadow-2xl outline-1 md:p-8"
    >
        {#if !generatedPassphrase}
            <div class="flex flex-col space-y-6">
                <h1 class="h2 text-center">Set Up Your Secure Account</h1>
                <div>
                    <p class="mb-8">
                        Choose a strong master password. This will be used to
                        encrypt your data.
                    </p>
                    <form
                        onsubmit={handleSetup}
                        use:enhance
                        class="flex flex-col"
                    >
                        <input
                            type="password"
                            bind:value={masterPassword}
                            placeholder="Master Password"
                            class="input mb-1"
                        />
                        <Progress value={passwordStrength} max={4}></Progress>
                        {#if passwordStrengthFeedback}
                            <div class="mt-2 text-sm">
                                {#if passwordStrengthFeedback.warning}
                                    <p class="text-warning-500">
                                        {passwordStrengthFeedback.warning}
                                    </p>
                                {/if}
                                <ul class="mt-1 list-disc space-y-1 pl-5">
                                    {#each passwordStrengthFeedback.suggestions as suggestion (suggestion)}
                                        <li>
                                            {suggestion}
                                        </li>
                                    {/each}
                                </ul>
                            </div>
                        {/if}
                        <button
                            type="submit"
                            disabled={isLoading || passwordStrength < 3}
                            class="btn mt-8 {passwordStrength < 3
                                ? 'preset-outlined-primary-500'
                                : 'preset-filled-primary-500'}"
                        >
                            {isLoading ? "Generating..." : "Create Account"}
                        </button>
                    </form>
                </div>
            </div>
        {:else}
            <div class="flex flex-col space-y-6">
                <h2 class="h2 text-center">Save Your Backup Passphrase</h2>

                <div
                    class="card alert preset-filled-warning-900-100 dark:preset-filled-warning-100-900 space-y-2 p-4"
                >
                    <TriangleAlert />
                    <div>
                        <h3 class="h3">Store this in a safe place!</h3>
                        <p>
                            This is the ONLY way to recover your account. Keep
                            it offline and secure.
                        </p>
                    </div>
                </div>

                <div
                    class="dark:preset-filled-surface-200-800 rounded-lg p-4 outline-1 dark:outline-0"
                >
                    <pre
                        class="whitespace-pre-wrap break-words text-center font-mono text-lg">{generatedPassphrase}</pre>
                    <button
                        class="btn preset-filled-surface-900-100 dark:preset-filled-surface-100-900 mt-2 w-full"
                        use:copy={{
                            text: generatedPassphrase,
                            successText: "Copied!",
                        }}
                        aria-label="Copy passphrase to clipboard"
                    >
                        Copy
                    </button>
                </div>

                <label class="flex cursor-pointer items-center space-x-3 p-2">
                    <input
                        type="checkbox"
                        class="checkbox"
                        bind:checked={confirmedSaved}
                    />
                    <span>I have securely saved my backup passphrase.</span>
                </label>

                <a
                    href="/dashboard"
                    class="btn w-full {confirmedSaved
                        ? 'preset-filled-primary-500'
                        : 'preset-outlined-surface-500 cursor-not-allowed'}"
                    aria-disabled={!confirmedSaved}
                    onclick={(e) => {
                        if (!confirmedSaved) e.preventDefault();
                    }}
                >
                    Continue to Dashboard
                </a>
            </div>
        {/if}

        {#if errorMessage}
            <p class="text-error-500 mt-4 text-center">{errorMessage}</p>
        {/if}
    </div>
</div>
