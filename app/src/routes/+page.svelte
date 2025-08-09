<script lang="ts">
    import {invoke} from "@tauri-apps/api/core";
    import {openUrl} from "@tauri-apps/plugin-opener";

    let userCode = $state<string>("");
    let status = $state<"before" | "waiting" | "done">("before");

    async function add_new_user() {
        try {
            let response: {
                user_profile_id: number;
                start_new_auth_flow_response: {
                    complete_uri: string;
                    user_code: string;
                    error_message: string | null;
                };
            } = await invoke("add_and_auth_new_user");

            console.log(response);

            if (response.start_new_auth_flow_response.error_message) {
                console.error(
                    "Error starting new auth flow:",
                    response.start_new_auth_flow_response.error_message
                );
                return;
            }

            userCode = response.start_new_auth_flow_response.user_code;
            await openUrl(response.start_new_auth_flow_response.complete_uri);

            try {
                status = "waiting";
                let wait_response: {
                    status: string;
                    error_message: string | null;
                } = await invoke("wait_for_user_auth", {
                    user_profile_id: response.user_profile_id,
                });

                if (wait_response.error_message) {
                    console.error(
                        "Error waiting for user auth:",
                        wait_response.error_message
                    );
                    status = "before"; // Reset status on error
                    return;
                }

                status = "done";
                console.log(
                    "User authenticated successfully:",
                    wait_response.status
                );
            } catch (e) {
                console.log("Error waiting for user auth:", e);
            }
        } catch (e) {
            console.log("Error adding new user:", e);
        }
    }
</script>

<div class="container mx-auto max-w-md p-8 space-y-8">
    <div class="card variant-filled-surface p-8 space-y-6">
        <header class="text-center space-y-4">
            <h1 class="h2 font-bold text-primary-500">Axonotes Authentication</h1>
            <p class="text-surface-600-300-token">Connect your account to get started</p>
        </header>

        <div class="space-y-6">
            {#if status === "before"}
                <div class="text-center space-y-4">
                    <div class="w-16 h-16 mx-auto bg-surface-200-700-token rounded-full flex items-center justify-center">
                        <svg class="w-8 h-8 text-primary-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"></path>
                        </svg>
                    </div>
                    <p class="text-surface-700-200-token">
                        Click the button below to start the authentication process.
                    </p>
                    <button
                        class="btn variant-filled-primary w-full"
                        onclick={add_new_user}
                    >
                        Start Authentication
                    </button>
                </div>
            {:else if status === "waiting"}
                <div class="text-center space-y-4">
                    <div class="w-16 h-16 mx-auto bg-warning-500 rounded-full flex items-center justify-center animate-pulse">
                        <svg class="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                        </svg>
                    </div>
                    <p class="text-surface-700-200-token">
                        Please complete the authentication in your browser.
                    </p>
                    <div class="card variant-ghost-warning p-4">
                        <p class="text-sm text-surface-600-300-token">Your user code is:</p>
                        <p class="code text-lg font-mono font-bold text-warning-600-300-token tracking-wider">
                            {userCode}
                        </p>
                    </div>
                </div>
            {:else if status === "done"}
                <div class="text-center space-y-4">
                    <div class="w-16 h-16 mx-auto bg-success-500 rounded-full flex items-center justify-center">
                        <svg class="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                        </svg>
                    </div>
                    <p class="h4 text-success-600-300-token font-semibold">
                        Authentication completed successfully!
                    </p>
                    <p class="text-surface-600-300-token">
                        You can now start using Axonotes.
                    </p>
                </div>
            {/if}
        </div>
    </div>
</div>