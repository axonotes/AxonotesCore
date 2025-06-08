<script lang="ts">
    import {page} from "$app/state";
    import {lockVault} from "$lib/client/cryptoStore";

    /**
     * Handles the logout process.
     * 1. Locks the client-side vault by deleting the key.
     * 2. Submits the form to trigger the server-side logout and external redirect.
     * The `|preventDefault` modifier on the button ensures this function
     * has full control over the submission timing.
     */
    async function handleLogout(event: MouseEvent) {
        console.log("Locking client-side vault before redirecting...");

        // 1. Perform the client-side cleanup first and wait for it to finish.
        await lockVault();

        console.log("Vault locked. Proceeding with server logout.");

        // 2. Find the form associated with the button and submit it.
        // This will now trigger the POST request to `/auth/logout`.
        const form = (event.target as HTMLButtonElement).form;
        if (form) {
            form.requestSubmit();
        }
    }
</script>

<!-- page.data.user will be defined in +layout.server.ts.
     Therefore this warning is fine -->
{#if page.data.user}
    <form action="/auth/logout" method="POST">
        <button
            type="submit"
            class="btn preset-filled-primary-500"
            on:click|preventDefault={handleLogout}
        >
            Logout
        </button>
    </form>
{:else}
    <a href="/auth/login" class="btn preset-filled-primary-500">Login</a>
{/if}
