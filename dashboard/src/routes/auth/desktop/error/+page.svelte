<script lang="ts">
    import {page} from "$app/state";
    import {TriangleAlert, RefreshCw, Home} from "@lucide/svelte";

    let errorType = $state("");
    let errorMessage = $state("");
    let errorDescription = $state("");

    $effect(() => {
        errorType = page.url.searchParams.get("error") || "unknown";

        switch (errorType) {
            case "invalid_request":
                errorMessage = "Invalid Request";
                errorDescription =
                    "The authentication request was invalid or malformed. This might be due to an expired link or corrupted data.";
                break;
            case "auth_failed":
                errorMessage = "Authentication Failed";
                errorDescription =
                    "The authentication process failed. This could be due to network issues or server problems.";
                break;
            case "session_expired":
                errorMessage = "Session Expired";
                errorDescription =
                    "Your authentication session has expired. Please start the authentication process again.";
                break;
            case "session_consumed":
                errorMessage = "Session Already Used";
                errorDescription =
                    "This authentication session has already been used. Please start a new authentication process.";
                break;
            default:
                errorMessage = "Unknown Error";
                errorDescription =
                    "An unexpected error occurred during the authentication process.";
        }
    });
</script>

<div class="grid w-full items-center lg:mt-24">
    <div
        class="card outline-error-500 dark:shadow-error-900 shadow-error-400 m-auto w-full max-w-xl p-4 shadow-2xl outline-1 md:p-8"
    >
        <div class="flex flex-col space-y-6">
            <div class="text-center">
                <TriangleAlert class="text-error-500 mx-auto mb-4 h-16 w-16" />
                <h1 class="h2 text-error-500">Authentication Error</h1>
                <h2 class="h3 mt-2">{errorMessage}</h2>
            </div>

            <div
                class="card alert preset-filled-error-500-100 dark:preset-filled-error-100-900 space-y-2 p-4"
            >
                <TriangleAlert class="h-5 w-5" />
                <div>
                    <h3 class="h4">What happened?</h3>
                    <p class="text-sm">{errorDescription}</p>
                </div>
            </div>

            <div class="space-y-4">
                <div class="card preset-filled-surface-100-800 p-4">
                    <h3 class="h4 mb-2">Troubleshooting Steps:</h3>
                    <ul class="list-disc space-y-1 pl-5 text-sm">
                        <li>Check your internet connection</li>
                        <li>Make sure your desktop app is up to date</li>
                        <li>Try clearing your browser cache</li>
                        <li>Close and reopen your desktop app</li>
                    </ul>
                </div>

                <div class="flex flex-col space-y-3">
                    <a
                        href="/auth/desktop"
                        class="btn preset-filled-primary-500 w-full"
                    >
                        <RefreshCw class="h-4 w-4" />
                        <span>Try Again</span>
                    </a>

                    <a
                        href="/dashboard"
                        class="btn preset-outlined-surface-500 w-full"
                    >
                        <Home class="h-4 w-4" />
                        <span>Go to Dashboard</span>
                    </a>
                </div>
            </div>

            <div class="border-surface-300-600 border-t pt-4">
                <p class="text-surface-500-400 text-center text-xs">
                    Error Code: <code class="code text-xs">{errorType}</code>
                </p>
                <p class="text-surface-500-400 mt-1 text-center text-xs">
                    Need help? Contact support with this error code.
                </p>
            </div>
        </div>
    </div>
</div>
