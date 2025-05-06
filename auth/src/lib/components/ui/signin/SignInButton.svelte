<script>
    import { Button } from "$lib/components/ui/button/index.js";
    import { SignIn } from "@auth/sveltekit/components";

    export let provider;
</script>

<Button class="w-full bg-transparent" variant="outline">
    <!-- Flex container for alignment and spacing -->
    <div class="inline-flex items-center justify-center gap-2 w-full">
        <!-- Slot for the icon (which should have height="1.5em") -->
        <slot />
        <SignIn provider={provider}/>
    </div>
</Button>
