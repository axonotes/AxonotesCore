<script lang="ts">
  import {app, isLoading} from "$lib/stores/app";
  import {Button} from "$lib/components/ui/button";
  import {Loader2, Lock} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  async function handleLogin() {
    await app.startLogin();
  }
</script>

<div class="bg-background flex h-full w-full items-center justify-center p-6">
  <div class="w-full max-w-sm space-y-8">
    <!-- Header -->
    <div class="space-y-4 text-center">
      <div
        class="bg-muted mx-auto flex h-12 w-12 items-center justify-center rounded-full"
      >
        <Lock class="text-muted-foreground h-5 w-5" />
      </div>
      <div class="space-y-2">
        <h1 class="text-3xl font-semibold tracking-tight">
          {m.auth_login_title()}
        </h1>
        <p class="text-muted-foreground text-sm">
          {m.auth_login_description()}
        </p>
      </div>
    </div>

    <!-- Login Button -->
    <div class="space-y-4">
      <Button class="h-10 w-full" disabled={$isLoading} onclick={handleLogin}>
        {#if $isLoading}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
          {m.auth_login_button_loading()}
        {:else}
          {m.auth_login_button_primary()}
        {/if}
      </Button>

      {#if $isLoading}
        <p class="text-muted-foreground text-center text-sm">
          {m.auth_login_hint()}
        </p>
      {/if}
    </div>
  </div>
</div>
