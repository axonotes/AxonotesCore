<script lang="ts">

    import {onMount} from "svelte";

    let isLoading = $state(true);

    onMount(async () => {
        try {

        } catch {

        } finally {
            isLoading = false;
        }

    });
</script>

<div class="h-full w-full">
    {#if isLoading}
        <div class="flex h-full w-full items-center justify-center">
            <div class="text-muted-foreground text-sm">Loading...</div>
        </div>
    {:else}
        <div class="flex h-full w-full items-center justify-center">
                Welcome to Axonotes!
        </div>
    {/if}
</div>

<!-- TODO: Add login form. This is the standard opening page for the app. Has title, subtitle, a login button
     TODO: and a use offline mode. Settings icon in the bottom right corner.
     TODO: Disregard offline mode and settings for now.
     TODO: When the login is pressed, check if the user has signed in (with WORKOS) with get_active_profile. if it's null -> auth with workos, else -> encryptionsetup.
     TODO: does_stdb_user_exist is used to check if user has completed encryption setup. if yes -> login, if not -> set master password.
     TODO: Recover password should be in settings option (barebones settings page, just add this for now). -->