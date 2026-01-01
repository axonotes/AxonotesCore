<script lang="ts">
  import {app, isLoading} from "$lib/stores/app";
  import {Button} from "$lib/components/ui/button";
  import {Card, CardContent, CardDescription, CardHeader, CardTitle} from "$lib/components/ui/card";
  import {Loader2} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  async function handleLogin() {
    await app.startLogin();
  }
</script>

<div
  class="flex min-h-screen items-center justify-center bg-gradient-to-br from-gray-50 to-gray-100 dark:from-gray-900 dark:to-gray-800"
>
  <Card class="mx-4 w-full max-w-md">
    <CardHeader class="space-y-1">
      <CardTitle class="text-center text-2xl font-bold">{m.login_title()}</CardTitle>
      <CardDescription class="text-center">{m.login_description()}</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <Button class="w-full" size="lg" onclick={handleLogin} disabled={$isLoading}>
        {#if $isLoading}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
          {m.login_authenticating()}
        {:else}
          {m.login_button()}
        {/if}
      </Button>

      {#if $isLoading}
        <p class="text-muted-foreground text-center text-sm">{m.login_check_browser()}</p>
      {/if}
    </CardContent>
  </Card>
</div>
