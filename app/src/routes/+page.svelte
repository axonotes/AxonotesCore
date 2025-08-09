<script lang="ts">
    import {invoke} from "@tauri-apps/api/core";
    import {openUrl} from "@tauri-apps/plugin-opener";

    interface User {
        user_profile_id: number;
        status: "authenticating" | "authenticated" | "error";
        userCode?: string;
        error?: string;
    }

    let users = $state<User[]>([]);
    let isAdding = $state<boolean>(false);

    async function addNewUser() {
        if (isAdding) return;

        isAdding = true;
        try {
            let response: {
                user_profile_id: number;
                start_new_auth_flow_response: {
                    complete_uri: string;
                    user_code: string;
                    error_message: string | null;
                };
            } = await invoke("add_and_auth_new_user");

            console.log("Add user response:", response);

            if (response.start_new_auth_flow_response.error_message) {
                console.error(
                    "Error starting new auth flow:",
                    response.start_new_auth_flow_response.error_message
                );
                return;
            }

            // Add user to list with authenticating status
            const newUser: User = {
                user_profile_id: response.user_profile_id,
                status: "authenticating",
                userCode: response.start_new_auth_flow_response.user_code,
            };
            users = [...users, newUser];

            // Open browser for authentication
            await openUrl(response.start_new_auth_flow_response.complete_uri);

            // Wait for authentication in background
            waitForAuth(response.user_profile_id);
        } catch (e) {
            console.error("Error adding new user:", e);
        } finally {
            isAdding = false;
        }
    }

    async function waitForAuth(userId: number) {
        try {
            let wait_response: {
                status: string;
                error_message: string | null;
            } = await invoke("wait_for_user_auth", {
                user_profile_id: userId,
            });

            // Update user status
            users = users.map((user) => {
                if (user.user_profile_id === userId) {
                    if (wait_response.error_message) {
                        return {
                            ...user,
                            status: "error",
                            error: wait_response.error_message,
                        };
                    } else {
                        return {...user, status: "authenticated"};
                    }
                }
                return user;
            });

            if (wait_response.error_message) {
                console.error(
                    "Auth error for user",
                    userId,
                    ":",
                    wait_response.error_message
                );
            } else {
                console.log(
                    "User",
                    userId,
                    "authenticated successfully:",
                    wait_response.status
                );
            }
        } catch (e) {
            console.error("Error waiting for user auth:", e);
            // Update user status to error
            users = users.map((user) => {
                if (user.user_profile_id === userId) {
                    return {...user, status: "error", error: String(e)};
                }
                return user;
            });
        }
    }

    async function removeUser(userId: number) {
        try {
            const success: boolean = await invoke("remove_user", {
                user_profile_id: userId,
            });

            if (success) {
                users = users.filter((user) => user.user_profile_id !== userId);
                console.log("User", userId, "removed successfully");
            }
        } catch (e) {
            console.error("Error removing user:", e);
            alert(`Failed to remove user: ${e}`);
        }
    }

    async function refreshUserToken(userId: number) {
        try {
            const success: boolean = await invoke("refresh_user_token", {
                user_profile_id: userId,
            });

            if (success) {
                console.log("Token refreshed successfully for user", userId);
                // Show success feedback
                const userIndex = users.findIndex(
                    (u) => u.user_profile_id === userId
                );
                if (userIndex !== -1) {
                    // Temporarily show success state (you could add a success field to User interface)
                    console.log("Refresh successful!");
                }
            }
        } catch (e) {
            console.error("Error refreshing token:", e);
            alert(`Failed to refresh token: ${e}`);
        }
    }
</script>

<div class="container mx-auto max-w-4xl space-y-8 p-8">
    <div class="card variant-filled-surface p-8">
        <header class="mb-8 space-y-4 text-center">
            <h1 class="h2 text-primary-500 font-bold">
                Axonotes User Management
            </h1>
            <p class="text-surface-600-300-token">
                Manage user accounts and authentication
            </p>
        </header>

        <!-- Add New User Section -->
        <div class="card variant-ghost-primary mb-8 p-6">
            <div class="flex items-center justify-between">
                <div>
                    <h3 class="h4 mb-2 font-semibold">Add New User</h3>
                    <p class="text-surface-600-300-token text-sm">
                        Start the authentication flow for a new user
                    </p>
                </div>
                <button
                    class="btn variant-filled-primary"
                    onclick={addNewUser}
                    disabled={isAdding}
                >
                    {#if isAdding}
                        <svg
                            class="mr-2 h-4 w-4 animate-spin"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                            ></path>
                        </svg>
                        Adding...
                    {:else}
                        <svg
                            class="mr-2 h-4 w-4"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M12 6v6m0 0v6m0-6h6m-6 0H6"
                            ></path>
                        </svg>
                        Add User
                    {/if}
                </button>
            </div>
        </div>

        <!-- Users List -->
        <div class="space-y-4">
            <h3 class="h4 font-semibold">Users ({users.length})</h3>

            {#if users.length === 0}
                <div class="card variant-ghost-surface p-8 text-center">
                    <div
                        class="bg-surface-200-700-token mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full"
                    >
                        <svg
                            class="text-surface-500 h-8 w-8"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197m13.5-9a2.5 2.5 0 11-5 0 2.5 2.5 0 015 0z"
                            ></path>
                        </svg>
                    </div>
                    <p class="text-surface-600-300-token">
                        No users added yet. Click "Add User" to get started.
                    </p>
                </div>
            {:else}
                {#each users as user}
                    <div class="card variant-ghost-surface p-6">
                        <div class="flex items-center justify-between">
                            <div class="flex items-center space-x-4">
                                <!-- Status Icon -->
                                <div
                                    class="flex h-10 w-10 items-center justify-center rounded-full {user.status ===
                                    'authenticated'
                                        ? 'bg-success-500'
                                        : user.status === 'authenticating'
                                          ? 'bg-warning-500 animate-pulse'
                                          : 'bg-error-500'}"
                                >
                                    {#if user.status === "authenticated"}
                                        <svg
                                            class="h-5 w-5 text-white"
                                            fill="none"
                                            stroke="currentColor"
                                            viewBox="0 0 24 24"
                                        >
                                            <path
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                                stroke-width="2"
                                                d="M5 13l4 4L19 7"
                                            ></path>
                                        </svg>
                                    {:else if user.status === "authenticating"}
                                        <svg
                                            class="h-5 w-5 text-white"
                                            fill="none"
                                            stroke="currentColor"
                                            viewBox="0 0 24 24"
                                        >
                                            <path
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                                stroke-width="2"
                                                d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                                            ></path>
                                        </svg>
                                    {:else}
                                        <svg
                                            class="h-5 w-5 text-white"
                                            fill="none"
                                            stroke="currentColor"
                                            viewBox="0 0 24 24"
                                        >
                                            <path
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                                stroke-width="2"
                                                d="M6 18L18 6M6 6l12 12"
                                            ></path>
                                        </svg>
                                    {/if}
                                </div>

                                <!-- User Info -->
                                <div>
                                    <h4 class="font-semibold">
                                        User ID: {user.user_profile_id}
                                    </h4>
                                    <p
                                        class="text-surface-600-300-token text-sm capitalize"
                                    >
                                        Status: {user.status}
                                    </p>
                                    {#if user.userCode && user.status === "authenticating"}
                                        <p
                                            class="code text-warning-600-300-token font-mono text-sm"
                                        >
                                            Code: {user.userCode}
                                        </p>
                                    {/if}
                                    {#if user.error}
                                        <p
                                            class="text-error-600-300-token text-sm"
                                        >
                                            Error: {user.error}
                                        </p>
                                    {/if}
                                </div>
                            </div>

                            <!-- Actions -->
                            <div class="flex items-center space-x-2">
                                {#if user.status === "authenticated"}
                                    <button
                                        class="btn btn-sm variant-filled-secondary"
                                        onclick={() =>
                                            refreshUserToken(
                                                user.user_profile_id
                                            )}
                                        title="Refresh Token"
                                    >
                                        <svg
                                            class="h-4 w-4"
                                            fill="none"
                                            stroke="currentColor"
                                            viewBox="0 0 24 24"
                                        >
                                            <path
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                                stroke-width="2"
                                                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                                            ></path>
                                        </svg>
                                        Refresh Token
                                    </button>
                                {/if}

                                <button
                                    class="btn btn-sm variant-filled-error"
                                    onclick={() =>
                                        removeUser(user.user_profile_id)}
                                    title="Remove User"
                                >
                                    <svg
                                        class="h-4 w-4"
                                        fill="none"
                                        stroke="currentColor"
                                        viewBox="0 0 24 24"
                                    >
                                        <path
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                            stroke-width="2"
                                            d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                                        ></path>
                                    </svg>
                                    Remove
                                </button>
                            </div>
                        </div>
                    </div>
                {/each}
            {/if}
        </div>
    </div>
</div>
