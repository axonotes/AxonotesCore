<script lang="ts">
	import '../app.css';
    import LightSwitch from "$lib/components/LightSwitch.svelte";
	
	let { children } = $props();
</script>

<div class="w-screen h-screen bg-red-500">
    <LightSwitch/>
    {@render children()}
</div>
