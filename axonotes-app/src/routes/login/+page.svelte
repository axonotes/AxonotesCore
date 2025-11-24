<script lang="ts">
  import {onMount} from "svelte";
  import {
    startLogin,
    isLoading,
    activeProfile,
    initAuth,
  } from "$lib/stores/authStore";
  import {Button} from "$lib/components/ui/button";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "$lib/components/ui/card";
  import {Loader2} from "@lucide/svelte";

  onMount(async () => {
    await initAuth();

    // Redirect if already logged in
    if ($activeProfile) {
      window.location.href = "/";
    }
  });

  async function handleLogin() {
    await startLogin();
  }
</script>

<div
  class="flex min-h-screen items-center justify-center bg-gradient-to-br from-gray-50 to-gray-100 dark:from-gray-900 dark:to-gray-800"
>
  <Card class="mx-4 w-full max-w-md">
    <CardHeader class="space-y-1">
      <CardTitle class="text-center text-2xl font-bold">Welcome Back</CardTitle>
      <CardDescription class="text-center">
        Sign in to your account to continue
      </CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <Button
        class="w-full"
        size="lg"
        onclick={handleLogin}
        disabled={$isLoading}
      >
        {#if $isLoading}
          <Loader2 class="mr-2 h-4 w-4 animate-spin" />
          Authenticating...
        {:else}
          Sign in with WorkOS
        {/if}
      </Button>

      {#if $isLoading}
        <p class="text-muted-foreground text-center text-sm">
          Check your browser to complete authentication
        </p>
      {/if}
    </CardContent>
  </Card>
</div>
