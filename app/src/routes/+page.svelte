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

<div>
    <header>
        <h1>Axonotes Desktop</h1>
        <p>Phase 2: Authentication Testing</p>
    </header>

    <main>
        <div>
            <h2>Authentication Status</h2>
            {#if status === "before"}
                <p>
                    Click the button below to start the authentication process.
                </p>
                <button onclick={add_new_user}>Start Authentication</button>
            {:else if status === "waiting"}
                <p>Please complete the authentication in your browser.</p>
                <p>Your user code is: <strong>{userCode}</strong></p>
            {:else if status === "done"}
                <p>Authentication completed successfully!</p>
            {/if}
        </div>
    </main>
</div>
