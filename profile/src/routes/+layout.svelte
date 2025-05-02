<script lang="ts">
	import '../app.css';
    import {ModeWatcher} from "mode-watcher";
    import LightSwitch from "$lib/components/ui/lightswitch/LightSwitch.svelte";

    let { children } = $props();
</script>

<ModeWatcher/>
<div class="absolute top-0 left-0 w-screen h-screen overflow-hidden">
    {@render children()}
</div>
