<script lang="ts">
    import "../app.css";
    import { ModeWatcher } from "mode-watcher";
    import LightSwitch from "$lib/components/ui/lightswitch/LightSwitch.svelte";
    let { children } = $props();
</script>

<ModeWatcher />
<div class="absolute w-screen h-screen overflow-hidden top-0 left-0">
    {@render children?.()}
</div>

<div class="absolute top-0 right-0 p-2">
    <LightSwitch/>
</div>

<div class="absolute top-0 left-0 px-4 py-2">
    <h2 class="text-2xl font-semibold">Athena Learning</h2>
</div>