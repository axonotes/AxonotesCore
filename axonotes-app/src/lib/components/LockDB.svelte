<script lang="ts">
    import Lock from "@lucide/svelte/icons/lock"
    import {DatabaseService} from "../services/database";
    import {Button} from "./ui/button";
    import {goto} from "$app/navigation";

    async function lockDB(): Promise<void> {
        await DatabaseService.lockDatabase();
        await goto('/unlock');
    }
</script>

<Button
    onclick={lockDB}
    variant="outline"
    size="icon"
    class="cursor-pointer"
>
    <Lock />
    <span class="sr-only">Lock database</span>
</Button>