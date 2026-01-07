<script lang="ts">
  import {app, activeProfile} from "$lib/stores/app";
  import {Button} from "$lib/components/ui/button";
  import * as Dialog from "$lib/components/ui/dialog";
  import {LogOut, ShieldCheck, User, Loader2} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  let logoutOpen = $state(false);
  let logoutLoading = $state(false);

  async function handleLogout() {
    if (!$activeProfile) return;

    logoutLoading = true;
    try {
      await app.logout($activeProfile.id);
      window.location.href = "/login";
    } catch {
      logoutLoading = false;
    }
  }
</script>

<div class="flex h-full w-full justify-center overflow-auto p-8">
  <div class="w-full max-w-md space-y-8">
    <!-- Header -->
    <div class="space-y-2">
      <h1 class="text-2xl leading-tight font-semibold tracking-tight">
        {m.settings_account_title()}
      </h1>
      <p class="text-muted-foreground text-sm leading-normal">
        {m.settings_account_description()}
      </p>
    </div>

    <!-- Profile Info -->
    {#if $activeProfile}
      <section class="space-y-4">
        <div class="space-y-4 rounded-lg border p-4">
          <div class="flex items-center gap-3">
            <div
              class="bg-muted flex h-10 w-10 items-center justify-center rounded-full"
            >
              <User class="text-muted-foreground h-5 w-5" />
            </div>
            <div>
              <p class="text-sm font-medium">{$activeProfile.name}</p>
              <p class="text-muted-foreground text-xs">
                {$activeProfile.email}
              </p>
            </div>
          </div>

          <!-- Privacy Note -->
          <div class="bg-muted/50 flex items-start gap-3 rounded-lg p-3">
            <ShieldCheck
              class="text-muted-foreground mt-0.5 h-4 w-4 shrink-0"
            />
            <p class="text-muted-foreground text-xs leading-normal">
              {m.settings_account_privacy_note()}
            </p>
          </div>
        </div>
      </section>
    {/if}

    <!-- Sign Out -->
    <section>
      <Button variant="outline" onclick={() => (logoutOpen = true)}>
        <LogOut class="mr-2 h-4 w-4" />
        {m.settings_account_logout()}
      </Button>
    </section>
  </div>
</div>

<!-- Logout Confirmation Modal -->
<Dialog.Root bind:open={logoutOpen}>
  <Dialog.Content class="max-w-xs">
    <Dialog.Header>
      <Dialog.Title>{m.settings_account_logout()}</Dialog.Title>
      <Dialog.Description
        >{m.settings_account_logout_confirm()}</Dialog.Description
      >
    </Dialog.Header>

    <Dialog.Footer class="flex-col gap-2 sm:flex-col">
      <Button
        variant="outline"
        class="w-full"
        onclick={() => (logoutOpen = false)}
      >
        {m.common_button_cancel()}
      </Button>
      <Button class="w-full" onclick={handleLogout} disabled={logoutLoading}>
        {#if logoutLoading}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
        {:else}
          <LogOut class="mr-2 h-4 w-4" />
        {/if}
        {m.settings_account_logout()}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
