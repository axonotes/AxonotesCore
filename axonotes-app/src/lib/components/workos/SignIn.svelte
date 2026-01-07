<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import { app, isLoading } from "$lib/stores/app";

    async function handleLogin() {
        await app.startLogin();
    }
</script>

<Button
        onclick={handleLogin}
        disabled={$isLoading}
        class="w-full"
>
    {#if $isLoading}
        Signing in...
    {:else}
        Sign in with WorkOS
    {/if}
</Button>