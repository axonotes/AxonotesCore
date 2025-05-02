<script>
    import {page} from "$app/state";
    import LightSwitch from "$lib/components/ui/lightswitch/LightSwitch.svelte";
    import {SignOut} from "@auth/sveltekit/components";
    import {Button} from "$lib/components/ui/button/index.js";

    $: username = page.data.session?.user?.name;
    $: email = page.data.session?.user?.email;
    $: pfp = page.data.session?.user?.image;
</script>

<div class="w-full h-full grid grid-cols-[auto_1fr]">
    <div class="w-72 border-r grid grid-rows-[1fr_auto]">
        <div class="h-full p-4">
            <h2 class="text-2xl font-semibold">Athena Learning</h2>
            <h4>Profile Settings</h4>

            <!-- TODO: Add settings for profile (spacetime is needed for that) -->
        </div>
        <div class="p-2 flex flex-row items-center justify-between pr-4">
            <LightSwitch/>
            <Button variant="secondary" class="h-8">
                <SignOut/>
            </Button>
        </div>
    </div>

    <div class="w-full grid  grid-rows-[auto_1fr]">
        <div class="h-20 w-full flex justify-end items-center border-b">
            <div class="w-44 h-14 text-sm flex flex-col justify-center items-end">
                <p class="font-semibold">{username}</p>
                <p class="font-light">{email}</p>
            </div>
            <div class="w-14 h-14 ml-2 mr-2 p-2">
                {#if pfp}
                    <img src={pfp} alt="profile" class="w-full h-full rounded-md">
                {/if}
            </div>
        </div>
        <div>
            <!-- TODO: Add settings sections -->
        </div>
    </div>
</div>