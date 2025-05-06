<script lang="ts">
    import * as Card from '$lib/components/ui/card';
    import {CheckCircle} from "@lucide/svelte";
</script>

<div
        class="flex min-h-[80vh] items-center justify-center bg-background p-4"
>
    <Card.Root class="w-full max-w-md text-center">
        <Card.Header>
            <Card.Title class="flex items-center justify-center gap-2 text-2xl">
                <CheckCircle class="h-6 w-6 text-green-600" />
                Authentication Successful!
            </Card.Title>
            <Card.Description>
                You have been successfully signed in.
            </Card.Description>
        </Card.Header>
        <Card.Content class="space-y-2">
            <p class="text-lg text-muted-foreground">
                You can now return to your application.
            </p>
            <p class="text-sm text-muted-foreground">
                (Your application is retrieving the necessary token in the
                background).
            </p>
        </Card.Content>
    </Card.Root>
</div>
