<script lang="ts">
    import {page} from '$app/state';
    import * as Card from '$lib/components/ui/card';
    import * as Alert from '$lib/components/ui/alert';
    import {Button} from '$lib/components/ui/button';
    import {AlertTriangle} from "@lucide/svelte";

    $: errorCode = page.url.searchParams.get('code');
    $: errorMessage = page.url.searchParams.get('message');
</script>

<div
        class="flex min-h-[80vh] items-center justify-center bg-background p-4"
>
    <Card.Root class="w-full max-w-md">
        <Card.Header>
            <Card.Title class="flex items-center gap-2 text-2xl">
                <AlertTriangle class="h-6 w-6 text-destructive"/>
                Authentication Error
            </Card.Title>
            <Card.Description>
                Something went wrong during the sign-in process.
            </Card.Description>
        </Card.Header>
        <Card.Content class="space-y-4">
            <Alert.Root variant="destructive">
                <!-- <AlertTriangle class="h-4 w-4" /> -->
                <!-- Icon already in Title, maybe redundant here -->
                <Alert.Title>Error Details</Alert.Title>
                <Alert.Description class="space-y-1">
                    {#if errorMessage}
                        <p>{errorMessage}</p>
                    {:else}
                        <p>An unspecified error occurred.</p>
                    {/if}
                    {#if errorCode}
                        <p class="text-xs text-muted-foreground">
                            Code: {errorCode}
                        </p>
                    {/if}
                </Alert.Description>
            </Alert.Root>
            <p class="text-sm text-muted-foreground">
                Please try initiating the login from your app again, or contact
                support if the problem persists.
            </p>
        </Card.Content>
    </Card.Root>
</div>
