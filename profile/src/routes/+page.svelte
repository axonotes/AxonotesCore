<script lang="ts">
    import {SignIn, SignOut} from "@auth/sveltekit/components";

    import {page} from "$app/state";
    import {Button} from "$lib/components/ui/button/index.js";
    import {devLog} from "$lib/utils";
    import {onMount} from "svelte";
    import {getSpacetimeJWT} from "$lib/auth/spacetime";

    async function connectToSpaceTimeDB(jwt: string | null) {
        if (!jwt) {
            devLog("No JWT");
            return;
        }

        devLog("Attempting to connect to SpaceTimeDB");

        try {

        } catch (e) {
            devLog("Error connecting to SpaceTimeDB: ", e);
        }
    }

    onMount(async () => {
        const {token, error} = await getSpacetimeJWT();

        if (error) {
            devLog("Error getting JWT: ", error);
            return;
        }

        await connectToSpaceTimeDB(token);
    })
</script>

<h1>Welcome to SvelteKit</h1>
<p>Visit <a href="https://svelte.dev/docs/kit">svelte.dev/docs/kit</a> to read the documentation</p>

<Button>
    <SignIn provider="github"/>
</Button>

<Button>
    <SignIn provider="google"/>
</Button>

<Button>
    <SignOut/>
</Button>

<p>{page.data.session?.user?.name}</p>
<p>{page.data.session?.user?.email}</p>
<p>{page.data.session?.user?.id}</p>