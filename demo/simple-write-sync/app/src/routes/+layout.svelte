<script lang="ts">
    import "../app.css";
    import LightSwitch from "$lib/components/LightSwitch.svelte";
    import {onMount} from "svelte";
    import {spacetimeClient} from "$lib/client/spacetime.svelte";

    let {children} = $props();

    onMount(() => {
        spacetimeClient.connect();
    })
</script>

<LightSwitch />
{@render children()}
