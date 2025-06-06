<script lang="ts">
    import {page} from "$app/state";
</script>

{#if page.data.user}
    <form action="/auth/logout" method="POST">
        <button type="submit" class="btn preset-filled-primary-500">Logout</button>
    </form>
{:else}
    <a href="/auth/login" class="btn preset-filled-primary-500">Login</a>
{/if}