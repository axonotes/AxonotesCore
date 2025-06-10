<script lang="ts">
    import {onMount} from "svelte";
    import {connectToSpacetime, spacetime} from "$lib/client/spacetime";
    import {goto} from "$app/navigation";

    onMount(() => {
        connectToSpacetime();
    });

    spacetime.subscribe((s) => {
        if (s.status === "connected" && s.connection) {
            const currentUser = Array.from(s.connection.db.user.iter()).find(
                (user) => user.identity.isEqual(s.connection!.identity!)
            );

            if (currentUser?.publicKey) {
                // User has a public key, so they need to unlock their account.
                goto("/auth/unlock");
            } else {
                // User needs to complete the initial encryption setup.
                goto("/auth/setup");
            }
        }
    });
</script>

<p>Loading...</p>
