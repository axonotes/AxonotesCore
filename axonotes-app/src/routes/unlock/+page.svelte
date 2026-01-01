<script lang="ts">
  import {app, isLoading, activeProfile} from "$lib/stores/app";
  import {Button} from "$lib/components/ui/button";
  import {Card, CardContent, CardDescription, CardHeader, CardTitle} from "$lib/components/ui/card";
  import {Input} from "$lib/components/ui/input";
  import {Label} from "$lib/components/ui/label";
  import {Loader2, Lock} from "@lucide/svelte";

  let password = $state("");
  let error = $state("");

  async function handleUnlock() {
    error = "";

    if (!password) {
      error = "Please enter your password";
      return;
    }

    try {
      await app.unlockDatabase(password);

      // Redirect after unlock
      if ($activeProfile) {
        window.location.href = "/";
      } else {
        window.location.href = "/login";
      }
    } catch (err) {
      console.error("Unlock error:", err);
      error = "Incorrect password. Please try again.";
    }
  }
</script>

<div
  class="flex min-h-screen items-center justify-center bg-gradient-to-br from-gray-50 to-gray-100 dark:from-gray-900 dark:to-gray-800"
>
  <Card class="mx-4 w-full max-w-md">
    <CardHeader class="space-y-1">
      <div class="flex items-center justify-center">
        <Lock class="mb-2 h-12 w-12 text-primary" />
      </div>
      <CardTitle class="text-center text-2xl font-bold">Unlock Database</CardTitle>
      <CardDescription class="text-center">
        Enter your password to continue
      </CardDescription>
    </CardHeader>
    <CardContent>
      <form
        onsubmit={(e) => {
          e.preventDefault();
          handleUnlock();
        }}
        class="space-y-4"
      >
        <div class="space-y-2">
          <Label for="password">Password</Label>
          <Input
            id="password"
            type="password"
            placeholder="Enter your password"
            bind:value={password}
            disabled={$isLoading}
            autofocus
          />
        </div>

        {#if error}
          <p class="text-sm text-destructive">{error}</p>
        {/if}

        <Button class="w-full" type="submit" disabled={$isLoading}>
          {#if $isLoading}
            <Loader2 class="mr-2 h-4 w-4 animate-spin" />
            Unlocking...
          {:else}
            Unlock
          {/if}
        </Button>
      </form>
    </CardContent>
  </Card>
</div>
