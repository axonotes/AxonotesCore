<script lang="ts">
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {slide} from "svelte/transition";
  import {Button} from "$lib/components/ui/button";
  import PasswordInput from "$lib/components/auth/PasswordInput.svelte";
  import {EncryptionService} from "$lib/services/encryption";
  import {app} from "$lib/stores/app";
  import {Download, Loader2} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  let password = $state("");
  let error = $state("");
  let isLoading = $state(false);

  async function handleSync() {
    error = "";

    if (!password) {
      error = m.auth_sync_error_incorrect();
      return;
    }

    isLoading = true;

    try {
      // Sync keys from server
      await EncryptionService.syncWithPassword(password);

      // Mark that we're entering local security setup
      // This prevents the layout from redirecting us to /app
      app.startLocalSecuritySetup();

      // Navigate to local security setup
      goto(resolve("/setup/local-security"), {replaceState: true});
    } catch (err) {
      console.error("Failed to sync keys:", err);
      error = m.auth_sync_error_incorrect();
    } finally {
      isLoading = false;
    }
  }
</script>

<div
  class="bg-background flex min-h-full w-full items-center justify-center p-6"
>
  <div class="w-full max-w-sm space-y-8">
    <!-- Header -->
    <div class="space-y-4 text-center">
      <div
        class="bg-muted mx-auto flex h-12 w-12 items-center justify-center rounded-full"
      >
        <Download class="text-muted-foreground h-5 w-5" />
      </div>
      <div class="space-y-2">
        <h1 class="text-3xl font-semibold tracking-tight">
          {m.auth_sync_title()}
        </h1>
        <p class="text-muted-foreground text-sm">
          {m.auth_sync_description()}
        </p>
      </div>
    </div>

    <!-- Form -->
    <form
      class="space-y-5"
      onsubmit={(e) => {
        e.preventDefault();
        handleSync();
      }}
    >
      <PasswordInput
        autofocus
        bind:value={password}
        disabled={isLoading}
        id="master-password"
        label={m.auth_sync_password_label()}
        placeholder={m.auth_sync_password_placeholder()}
      />

      {#if error}
        <div transition:slide={{duration: 150}}>
          <p class="text-destructive text-center text-sm">{error}</p>
        </div>
      {/if}

      <Button
        class="h-10 w-full"
        disabled={isLoading || !password}
        type="submit"
      >
        {#if isLoading}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
          {m.auth_sync_button_loading()}
        {:else}
          {m.auth_sync_button_primary()}
        {/if}
      </Button>

      <button
        class="text-muted-foreground/70 hover:text-muted-foreground mx-auto block text-sm transition-colors disabled:pointer-events-none"
        disabled={isLoading}
        onclick={() => goto(resolve("/sync/recovery"))}
        type="button"
      >
        {m.auth_sync_link_forgot()}
      </button>
    </form>
  </div>
</div>
