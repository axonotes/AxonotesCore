<script lang="ts">
    import {page} from "$app/state";
    import {onMount} from "svelte";
    import {copy} from "$lib/actions/copy";
    import {CheckCircle, ExternalLink, Copy, RotateCcw} from "@lucide/svelte";

    let sessionId = $state("");
    let desktopUrl = $state("");
    let launched = $state(false);
    let error = $state("");
    let isRetrying = $state(false);

    onMount(() => {
        sessionId = page.url.searchParams.get("session_id") || "";
        desktopUrl = page.url.searchParams.get("desktop_url") || "";

        if (desktopUrl) {
            attemptDesktopLaunch();
        }
    });

    async function attemptDesktopLaunch() {
        isRetrying = true;
        error = "";

        try {
            // Attempt to launch desktop app
            window.location.href = desktopUrl;
            launched = true;

            // Fallback: Show manual instructions after a delay
            setTimeout(() => {
                if (!launched) {
                    error = "Unable to automatically launch desktop app";
                }
            }, 3000);
        } catch (err) {
            error = "Failed to launch desktop app";
            console.error("Desktop launch failed:", err);
        } finally {
            isRetrying = false;
        }
    }
</script>

<div class="grid w-full items-center lg:mt-24">
    <div
        class="card outline-primary-500 dark:shadow-primary-900 shadow-primary-400 m-auto w-full max-w-xl p-4 shadow-2xl outline-1 md:p-8"
    >
        <div class="flex flex-col space-y-6">
            <div class="text-center">
                <CheckCircle class="text-success-500 mx-auto mb-4 h-16 w-16" />
                <h1 class="h2">Authentication Successful!</h1>
                <p class="text-surface-600-400 mt-2">
                    Your desktop app should open automatically.
                </p>
            </div>

            {#if launched && !error}
                <div
                    class="card alert preset-filled-success-500-100 dark:preset-filled-success-100-900 space-y-2 p-4"
                >
                    <CheckCircle class="h-5 w-5" />
                    <div>
                        <h3 class="h4">Desktop app launched successfully!</h3>
                        <p class="text-sm">
                            If the app didn't open, you can manually copy the
                            link below or try again.
                        </p>
                    </div>
                </div>
            {/if}

            {#if error}
                <div
                    class="card alert preset-filled-warning-500-100 dark:preset-filled-warning-100-900 space-y-2 p-4"
                >
                    <ExternalLink class="h-5 w-5" />
                    <div>
                        <h3 class="h4">Manual Launch Required</h3>
                        <p class="text-sm">{error}</p>
                    </div>
                </div>
            {/if}

            <div class="space-y-4">
                <div>
                    <label class="label mb-2">
                        <span class="text-sm font-medium"
                            >Desktop App Link:</span
                        >
                    </label>
                    <div
                        class="input-group input-group-divider grid-cols-[1fr_auto]"
                    >
                        <input
                            type="text"
                            readonly
                            value={desktopUrl}
                            class="input bg-surface-100-800 text-sm"
                        />
                        <button
                            class="btn preset-filled-surface-900-100 dark:preset-filled-surface-100-900"
                            use:copy={{
                                text: desktopUrl,
                                successText: "Copied!",
                            }}
                            aria-label="Copy desktop link to clipboard"
                        >
                            <Copy class="h-4 w-4" />
                        </button>
                    </div>
                </div>

                <div class="flex flex-col space-y-3">
                    <button
                        onclick={attemptDesktopLaunch}
                        disabled={isRetrying}
                        class="btn preset-filled-primary-500 w-full"
                    >
                        {#if isRetrying}
                            <span class="animate-spin">⟳</span>
                            <span>Launching...</span>
                        {:else}
                            <RotateCcw class="h-4 w-4" />
                            <span>Try Again</span>
                        {/if}
                    </button>

                    <a
                        href="/dashboard"
                        class="btn preset-outlined-surface-500 w-full"
                    >
                        Continue to Dashboard
                    </a>
                </div>
            </div>

            <div class="border-surface-300-600 border-t pt-4">
                <p class="text-surface-500-400 text-center text-xs">
                    Session ID: <code class="code text-xs">{sessionId}</code>
                </p>
            </div>
        </div>
    </div>
</div>
