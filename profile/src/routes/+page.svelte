<script lang="ts">
    import {onMount} from "svelte";
    import {page} from "$app/state";
    import {goto} from "$app/navigation";

    onMount(() => {
        if (page.data.session) {
            goto("/profile");
        } else {
            goto("/auth");
        }
    })
</script>