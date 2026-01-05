<script lang="ts">
  import {app, isLoading} from "$lib/stores/app";
  import {Button} from "$lib/components/ui/button";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
    CardFooter,
  } from "$lib/components/ui/card";
  import {Input} from "$lib/components/ui/input";
  import {Label} from "$lib/components/ui/label";
  import {Loader2, Lock, Mail, ArrowRight} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  async function handleLogin() {
    await app.startLogin();
    console.log("1");
  }
</script>

<div
  class="from-background to-muted/20 flex min-h-screen items-center justify-center bg-gradient-to-br p-4"
>
  <div class="w-full max-w-md space-y-6">
    <div class="flex flex-col items-center space-y-2 text-center">
      <div class="bg-primary/10 rounded-full p-3">
        <Lock class="text-primary h-6 w-6" />
      </div>
      <h1 class="text-3xl font-bold tracking-tight">{m.login_title()}</h1>
      <p class="text-muted-foreground">{m.login_description()}</p>
    </div>

    <Card class="border-border/50 shadow-lg">
      <CardContent class="pt-6">
        <Button
          class="h-12 w-full text-base font-medium transition-all duration-200 hover:shadow-md"
          onclick={handleLogin}
          disabled={$isLoading}
          variant="outline"
        >
          {#if $isLoading}
            <Loader2 class="mr-2 h-4 w-4 animate-spin" />
            {m.login_authenticating()}
          {:else}
            <svg class="mr-2 h-5 w-5" viewBox="0 0 24 24" fill="currentColor">
              <path
                d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm0 22c-5.514 0-10-4.486-10-10S6.486 2 12 2s10 4.486 10 10-4.486 10-10 10z"
              />
              <path
                d="M12 4.5c-4.136 0-7.5 3.364-7.5 7.5s3.364 7.5 7.5 7.5 7.5-3.364 7.5-7.5-3.364-7.5-7.5-7.5zm0 13.5c-3.309 0-6-2.691-6-6s2.691-6 6-6 6 2.691 6 6-2.691 6-6 6z"
              />
              <path
                d="M12 6c-3.309 0-6 2.691-6 6s2.691 6 6 6 6-2.691 6-6-2.691-6-6-6zm0 10.5c-2.485 0-4.5-2.015-4.5-4.5S9.515 7.5 12 7.5s4.5 2.015 4.5 4.5-2.015 4.5-4.5 4.5z"
              />
            </svg>
            {m.login_button()}
          {/if}
        </Button>
      </CardContent>

      {#if $isLoading}
        <CardFooter
          class="flex flex-col items-center justify-center space-y-2 p-6 pt-0"
        >
          <p class="text-muted-foreground text-center text-sm">
            <Loader2 class="mr-1 inline h-3 w-3 animate-spin" />
            {m.login_check_browser()}
          </p>
        </CardFooter>
      {/if}
    </Card>

    <p class="text-muted-foreground px-8 text-center text-sm">
      By continuing, you agree to our Terms of Service and Privacy Policy.
    </p>
  </div>
</div>
