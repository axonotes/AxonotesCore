<script lang="ts">
    import {Moon, Sun} from "@lucide/svelte";

    let checked = $state(false);

    $effect(() => {
        const mode = localStorage.getItem('mode') || 'light';
        checked = mode === 'dark';
    });

    interface CheckedChangeEvent {
        checked: boolean;
    }

    const onCheckedChange = (event: CheckedChangeEvent) => {
        const mode = event.checked ? 'dark' : 'light';
        document.documentElement.setAttribute('data-mode', mode);
        try {
            localStorage.setItem('mode', mode);
        } catch (error) {
            console.warn('Unable to save mode to localStorage:', error);
        }
        checked = event.checked;
    };
</script>

<svelte:head>
    <script>
        try {
            const mode = localStorage.getItem('mode') || 'light';
            document.documentElement.setAttribute('data-mode', mode);
        } catch (error) {
            console.warn('Unable to load mode from localStorage:', error);
            document.documentElement.setAttribute('data-mode', 'light');
        }
    </script>
</svelte:head>

<div class="relative">
    {#if checked}
        <button
                type="button"
                class="btn-icon preset-filled scheme-light"
                onclick={() => onCheckedChange({checked: false})}
                aria-label="Switch to light mode"
        >
            <Sun/>
        </button>
    {:else}
        <button
                type="button"
                class="btn-icon preset-filled scheme-dark"
                onclick={() => onCheckedChange({checked: true})}
                aria-label="Switch to dark mode"
        >
            <Moon/>
        </button>
    {/if}
</div>