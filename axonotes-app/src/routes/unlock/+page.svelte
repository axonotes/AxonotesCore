<script lang="ts">
    import { onMount } from "svelte";
    import { getCurrentUnlockMode, setDatabaseEncryption, unlockDatabase, type UnlockMode } from "$lib/services/database";
    import UnlockPassword from "$lib/components/unlocker/UnlockPassword.svelte";
    import UnlockPin from "$lib/components/unlocker/UnlockPin.svelte";

    let currentMode = $state<UnlockMode>("pass");
    let isLoading = $state(true);

    onMount(async () => {
        try {
            currentMode = await getCurrentUnlockMode();
            if (currentMode === 'none') {
                await unlockDatabase("");
                await setDatabaseEncryption("A1A1A1A1", 'pin');
            }
        } catch (err) {
            console.error("Failed to get unlock mode:", err);
            // Default to password if we can't determine
            currentMode = "pass";
        } finally {
            isLoading = false;
        }
    });

    function handleModeChange(newMode: UnlockMode) {
        currentMode = newMode;
    }
</script>

<div class="h-full w-full">
    {#if isLoading}
        <div class="flex h-full w-full items-center justify-center">
            <div class="text-muted-foreground text-sm">Loading...</div>
        </div>
    {:else if currentMode === "pin"}
        <UnlockPin changeUnlockMode={handleModeChange} />
    {:else if currentMode === "pass"}
        <UnlockPassword changeUnlockMode={handleModeChange} />
    {/if}
</div>