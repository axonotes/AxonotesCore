<script lang="ts">
    import "../app.css";
    import LightSwitch from "$lib/components/LightSwitch.svelte";

    let {children} = $props();
</script>

<LightSwitch/>
{@render children()}
