<script lang="ts">
    import {enhance} from "$app/forms";
    import {User, LogOut, ArrowRight} from "@lucide/svelte";

    let {data} = $props();
    let isLoading = $state(false);
</script>

<div class="grid w-full items-center lg:mt-24">
    <div
        class="card outline-primary-500 dark:shadow-primary-900 shadow-primary-400 m-auto w-full max-w-xl p-4 shadow-2xl outline-1 md:p-8"
    >
        <div class="flex flex-col space-y-6">
            <div class="text-center">
                <h1 class="h2 mb-2">Device Authorization</h1>
                <p class="text-surface-600 dark:text-surface-400">
                    Authorize your desktop application
                </p>
            </div>

            <!-- User Code Display -->
            <div class="text-center">
                <p class="text-surface-600 dark:text-surface-400 mb-2 text-sm">
                    Device Code:
                </p>
                <div class="card preset-filled-surface-200-800 rounded-lg p-4">
                    <code class="font-mono text-2xl font-bold tracking-wider">
                        {data.user_code}
                    </code>
                </div>
            </div>

            {#if data.currentUser}
                <!-- User is logged in - show account selection -->
                <div class="space-y-4">
                    <div
                        class="card preset-filled-surface-100-900 rounded-lg p-4"
                    >
                        <div class="flex items-center space-x-3">
                            <div
                                class="bg-primary-500 flex h-10 w-10 items-center justify-center rounded-full"
                            >
                                <User class="h-5 w-5 text-white" />
                            </div>
                            <div>
                                <p class="font-medium">
                                    {data.currentUser.fullName ||
                                        data.currentUser.email}
                                </p>
                                <p
                                    class="text-surface-600 dark:text-surface-400 text-sm"
                                >
                                    {data.currentUser.email}
                                </p>
                            </div>
                        </div>
                    </div>

                    <div class="space-y-3">
                        <!-- Use Current Account -->
                        <form
                            method="POST"
                            action="?/useCurrentAccount"
                            use:enhance={() => {
                                isLoading = true;
                                return async ({update}) => {
                                    await update();
                                    isLoading = false;
                                };
                            }}
                        >
                            <input
                                type="hidden"
                                name="user_code"
                                value={data.user_code}
                            />
                            <button
                                type="submit"
                                disabled={isLoading}
                                class="btn preset-filled-primary-500 flex w-full items-center justify-center space-x-2"
                            >
                                {#if isLoading}
                                    <div
                                        class="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent"
                                    ></div>
                                    <span>Authorizing...</span>
                                {:else}
                                    <ArrowRight class="h-4 w-4" />
                                    <span>Use This Account</span>
                                {/if}
                            </button>
                        </form>

                        <!-- Use Other Account -->
                        <form
                            method="POST"
                            action="?/useOtherAccount"
                            use:enhance={() => {
                                isLoading = true;
                                return async ({update}) => {
                                    await update();
                                    isLoading = false;
                                };
                            }}
                        >
                            <input
                                type="hidden"
                                name="user_code"
                                value={data.user_code}
                            />
                            <button
                                type="submit"
                                disabled={isLoading}
                                class="btn preset-outlined-surface-500 flex w-full items-center justify-center space-x-2"
                            >
                                <LogOut class="h-4 w-4" />
                                <span>Use Other Account</span>
                            </button>
                        </form>
                    </div>
                </div>
            {:else}
                <!-- User not logged in - show login button -->
                <form
                    method="POST"
                    action="?/loginWithUserCode"
                    use:enhance={() => {
                        isLoading = true;
                        return async ({update}) => {
                            await update();
                            isLoading = false;
                        };
                    }}
                >
                    <input
                        type="hidden"
                        name="user_code"
                        value={data.user_code}
                    />
                    <button
                        type="submit"
                        disabled={isLoading}
                        class="btn preset-filled-primary-500 flex w-full items-center justify-center space-x-2"
                    >
                        {#if isLoading}
                            <div
                                class="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent"
                            ></div>
                            <span>Redirecting...</span>
                        {:else}
                            <ArrowRight class="h-4 w-4" />
                            <span>Sign In to Authorize</span>
                        {/if}
                    </button>
                </form>
            {/if}

            <div class="text-center">
                <p class="text-surface-600 dark:text-surface-400 text-sm">
                    Make sure the device code above matches the one shown in
                    your desktop application.
                </p>
            </div>
        </div>
    </div>
</div>
