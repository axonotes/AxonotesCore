<script lang="ts">
    import {page} from "$app/state";
    import {AlertCircle} from "@lucide/svelte";

    const error = page.url.searchParams.get("error");

    const errorMessages = {
        missing_user_code: {
            title: "Missing Device Code",
            description:
                "No device code was provided. Please try starting the authorization process again from your desktop application.",
            action: "retry",
        },
        invalid_user_code: {
            title: "Invalid Device Code",
            description:
                "The device code is invalid or has expired. Please try starting the authorization process again from your desktop application.",
            action: "retry",
        },
        invalid_request: {
            title: "Invalid Request",
            description:
                "The authorization request is malformed. Please try starting the authorization process again from your desktop application.",
            action: "retry",
        },
        auth_failed: {
            title: "Authentication Failed",
            description:
                "The authentication process failed. Please try again or contact support if the problem persists.",
            action: "retry",
        },
        not_authenticated: {
            title: "Not Authenticated",
            description:
                "You need to be logged in to complete this action. Please try the authorization process again.",
            action: "login",
        },
        invalid_token: {
            title: "Invalid Session",
            description:
                "Your session has expired or is invalid. Please try the authorization process again.",
            action: "login",
        },
        completion_failed: {
            title: "Authorization Failed",
            description:
                "Failed to complete the device authorization. Please try again or contact support if the problem persists.",
            action: "retry",
        },
        expired_token: {
            title: "Code Expired",
            description:
                "The device code has expired. Please start a new authorization process from your desktop application.",
            action: "retry",
        },
        access_denied: {
            title: "Access Denied",
            description:
                "You denied the authorization request. If this was a mistake, please try again.",
            action: "retry",
        },
    };

    const currentError = errorMessages[error as keyof typeof errorMessages] || {
        title: "Unknown Error",
        description:
            "An unexpected error occurred. Please try again or contact support if the problem persists.",
        action: "retry",
    };
</script>

<svelte:head>
    <title>Device Authorization Error</title>
</svelte:head>

<div class="grid w-full items-center lg:mt-24">
    <div
        class="card outline-error-500 dark:shadow-error-900 shadow-error-400 m-auto w-full max-w-xl p-4 shadow-2xl outline-1 md:p-8"
    >
        <div class="flex flex-col items-center space-y-6 text-center">
            <AlertCircle class="text-error-500 h-16 w-16" />

            <div>
                <h1 class="h2 text-error-500 mb-2">{currentError.title}</h1>
                <p class="text-surface-600 dark:text-surface-400">
                    {currentError.description}
                </p>
            </div>

            {#if error}
                <div
                    class="card preset-filled-error-100-900 w-full rounded-lg p-4"
                >
                    <p class="text-sm">
                        <strong>Error Code:</strong>
                        {error}
                    </p>
                </div>
            {/if}

            <div class="text-surface-600 dark:text-surface-400 text-xs">
                <p>
                    If you continue to experience issues, please contact
                    support.
                </p>
            </div>
        </div>
    </div>
</div>
