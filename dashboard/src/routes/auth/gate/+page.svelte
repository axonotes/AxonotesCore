<script lang="ts">
    import {onMount} from "svelte";
    import {connectToSpacetime, spacetime} from "$lib/client/spacetime";
    import {goto} from "$app/navigation";
    import {page} from "$app/state";

    const user_code = page.url.searchParams.get("user_code");

    onMount(() => {
        connectToSpacetime();
    });

    spacetime.subscribe((s) => {
        if (s.status === "connected" && s.connection) {
            const currentUser = Array.from(s.connection.db.user.iter()).find(
                (user) => user.identity.isEqual(s.connection!.identity!)
            );

            let user_code_param = "";

            if (user_code) {
                user_code_param = `?user_code=${user_code}`;
            }

            if (currentUser?.publicKey) {
                if (user_code) {
                    // User has a public key, complete device flow immediately
                    goto(`/auth/device/complete${user_code_param}`);
                } else {
                    // User has a public key, so they need to unlock their account.
                    goto("/auth/unlock");
                }
            } else {
                // User needs to complete the initial encryption setup
                goto(`/auth/setup${user_code_param}`);
            }
        }
    });
</script>

<div class="grid w-full items-center lg:mt-24">
    <div
        class="card outline-primary-500 dark:shadow-primary-900 shadow-primary-400 m-auto w-full max-w-xl p-4 shadow-2xl outline-1 md:p-8"
    >
        <div class="flex flex-col items-center space-y-6">
            <div
                class="border-primary-500 h-8 w-8 animate-spin rounded-full border-2 border-t-transparent"
            ></div>
            <p class="text-surface-600 dark:text-surface-400 text-center">
                Setting up your secure connection...
            </p>
        </div>
    </div>
</div>
