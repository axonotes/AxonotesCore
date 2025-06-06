<script lang="ts">
    import '../app.css';
    import LightSwitch from "$lib/components/LightSwitch.svelte";
    import AuthButton from "$lib/components/AuthButton.svelte";

    let {children} = $props();
</script>

<div class="flex min-h-screen flex-col bg-gradient-to-br from-primary-700/10 to-transparent dark:from-transparent dark:to-primary-700/20">
    <header class="absolute z-20 top-0 left-0 w-full flex justify-end items-center p-2">
        <AuthButton/>
        <LightSwitch/>
    </header>

    <main class="flex-grow container mx-auto p-4 sm:p-6 lg:p-8">
        {@render children()}
    </main>

    <footer class="p-4 text-center text-sm">
        © {new Date().getFullYear()} Axonotes. All rights reserved.
    </footer>
</div>