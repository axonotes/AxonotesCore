<script lang="ts">
    import {onMount} from "svelte";
    import {ExternalLink, Loader2} from "@lucide/svelte";

    let isLoading = $state(true);
    let error = $state("");

    onMount(() => {
        // The server should redirect immediately, but if we're here, something went wrong
        setTimeout(() => {
            if (isLoading) {
                error = "Authentication initiation failed. Please try again.";
                isLoading = false;
            }
        }, 5000);
    });
</script>

<div class="grid w-full items-center lg:mt-24">
    <div
        class="card outline-primary-500 dark:shadow-primary-900 shadow-primary-400 m-auto w-full max-w-xl p-4 shadow-2xl outline-1 md:p-8"
    >
        <div class="flex flex-col space-y-6">
            {#if isLoading}
                <div class="text-center">
                    <Loader2
                        class="text-primary-500 mx-auto mb-4 h-16 w-16 animate-spin"
                    />
                    <h1 class="h2">Starting Authentication</h1>
                    <p class="text-surface-600-400 mt-2">
                        Redirecting you to the authentication page...
                    </p>
                </div>

                <div
                    class="card alert preset-filled-primary-500-100 dark:preset-filled-primary-100-900 space-y-2 p-4"
                >
                    <ExternalLink class="h-5 w-5" />
                    <div>
                        <h3 class="h4">What's happening?</h3>
                        <p class="text-sm">
                            You're being redirected to authenticate with your
                            account. This process will connect your desktop app
                            to your account.
                        </p>
                    </div>
                </div>
            {:else}
                <div class="text-center">
                    <h1 class="h2 text-error-500">Authentication Failed</h1>
                    <p class="text-surface-600-400 mt-2">{error}</p>
                </div>

                <div class="flex flex-col space-y-3">
                    <a
                        href="/auth/desktop"
                        class="btn preset-filled-primary-500 w-full"
                    >
                        Try Again
                    </a>

                    <a
                        href="/dashboard"
                        class="btn preset-outlined-surface-500 w-full"
                    >
                        Go to Dashboard
                    </a>
                </div>
            {/if}
        </div>
    </div>
</div>
