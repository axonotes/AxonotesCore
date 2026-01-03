<script lang="ts">
  import {app, isLoading, activeProfile} from "$lib/stores/app";
  import {Button} from "$lib/components/ui/button";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "$lib/components/ui/card";
  import {Input} from "$lib/components/ui/input";
  import {Label} from "$lib/components/ui/label";
  import {Badge} from "$lib/components/ui/badge";
  import {Alert, AlertDescription} from "$lib/components/ui/alert";
  import {Loader2, Lock, KeyRound, Hash} from "@lucide/svelte";
  import type {UnlockMode} from "$lib/services/database";

  let currentMode = $state<UnlockMode>("pass");
  let password = $state("");
  let pinDigits = $state<string[]>(Array(8).fill(""));
  let error = $state("");
  let pinInputs: HTMLInputElement[] = [];

  function toggleMode() {
    currentMode = currentMode === "pass" ? "pin" : "pass";
    password = "";
    pinDigits = Array(8).fill("");
    error = "";
  }

  function handlePinInput(index: number, event: Event) {
    const input = event.target as HTMLInputElement;
    const value = input.value.toUpperCase();

    // Only allow 0-9 and A-Z
    if (value && !/^[0-9A-Z]$/.test(value)) {
      input.value = "";
      return;
    }

    pinDigits[index] = value;

    // Auto-advance to next input
    if (value && index < 7) {
      pinInputs[index + 1]?.focus();
    }

    // Update combined password
    password = pinDigits.join("");
  }

  function handlePinKeydown(index: number, event: KeyboardEvent) {
    // Handle backspace
    if (event.key === "Backspace" && !pinDigits[index] && index > 0) {
      pinInputs[index - 1]?.focus();
    }
  }

  function handlePinPaste(event: ClipboardEvent) {
    // Handle clipboard paste
    event.preventDefault();
    const pastedData = event.clipboardData?.getData("text").toUpperCase() || "";
    const chars = pastedData.slice(0, 8).split("");

    // Validate all characters
    if (chars.every((char) => /^[0-9A-Z]$/.test(char))) {
      chars.forEach((char, i) => {
        pinDigits[i] = char;
        if (pinInputs[i]) {
          pinInputs[i].value = char;
        }
      });
      password = pinDigits.join("");

      // Focus the next empty input or the last one
      const nextEmptyIndex = chars.length < 8 ? chars.length : 7;
      pinInputs[nextEmptyIndex]?.focus();
    }
  }

  async function handleUnlock() {
    error = "";

    if (!password) {
      error =
        currentMode === "pin"
          ? "Please enter your PIN"
          : "Please enter your password";
      return;
    }

    if (
      currentMode === "pin" &&
      (password.length !== 8 || !/^[0-9A-Z]+$/.test(password))
    ) {
      error = "PIN must be exactly 8 characters (0-9, A-Z uppercase only)";
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
      error = `Incorrect ${currentMode === "pin" ? "PIN" : "password"}. Please try again.`;
      password = "";

      // Clear PIN inputs on error
      if (currentMode === "pin") {
        pinDigits = Array(8).fill("");
        pinInputs.forEach((input) => {
          if (input) input.value = "";
        });
        pinInputs[0]?.focus();
      }
    }
  }
</script>

<div class="bg-background flex min-h-screen items-center justify-center p-4">
  <Card class="relative w-full max-w-md pb-14">
    <CardHeader class="space-y-3">
      <div class="flex items-center justify-center">
        <div class="bg-primary/10 rounded-full p-3">
          <Lock class="text-primary h-8 w-8" />
        </div>
      </div>

      <div class="space-y-1">
        <CardTitle class="text-center text-2xl font-bold"
          >Welcome Back</CardTitle
        >
        <CardDescription class="text-center">
          Please unlock your database to continue
        </CardDescription>
      </div>

      <div class="flex justify-center">
        <Badge
          variant="outline"
          class="border-primary/20 bg-primary/10 text-primary"
        >
          <div class="flex items-center gap-1.5">
            {#if currentMode === "pin"}
              <Hash class="h-3 w-3" />
              PIN
            {:else}
              <KeyRound class="h-3 w-3" />
              Password
            {/if}
          </div>
        </Badge>
      </div>
    </CardHeader>

    <CardContent>
      <form
        onsubmit={(e) => {
          e.preventDefault();
          handleUnlock();
        }}
        class="space-y-4"
      >
        {#if currentMode === "pin"}
          <!-- PIN Input with 8 boxes -->
          <div class="space-y-2">
            <Label>Enter your PIN</Label>
            <div class="flex justify-center gap-2" onpaste={handlePinPaste}>
              {#each Array(8) as _, i (i)}
                <input
                  bind:this={pinInputs[i]}
                  type="text"
                  maxlength="1"
                  class="border-input bg-background focus:ring-ring h-12 w-10 rounded-md border text-center font-mono text-lg uppercase focus:ring-2 focus:ring-offset-2 focus:outline-none disabled:cursor-not-allowed disabled:opacity-50"
                  disabled={$isLoading}
                  autofocus={i === 0}
                  oninput={(e) => handlePinInput(i, e)}
                  onkeydown={(e) => handlePinKeydown(i, e)}
                />
              {/each}
            </div>
            <p class="text-muted-foreground text-center text-xs">
              PIN must be exactly 8 characters using only 0-9 and A-Z
              (uppercase)
            </p>
          </div>
        {:else}
          <!-- Password Input -->
          <div class="space-y-2">
            <Label for="password">Enter your password</Label>
            <Input
              id="password"
              type="password"
              placeholder="Enter your password"
              bind:value={password}
              disabled={$isLoading}
              autofocus
              class="font-mono"
            />
          </div>
        {/if}

        {#if error}
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        {/if}

        <Button class="w-full" type="submit" disabled={$isLoading}>
          {#if $isLoading}
            <Loader2 class="mr-2 h-4 w-4 animate-spin" />
            Unlocking...
          {:else}
            <Lock class="mr-2 h-4 w-4" />
            Unlock Database
          {/if}
        </Button>
      </form>
    </CardContent>

    <!-- Mode toggle button -->
    <div class="absolute right-4 bottom-4">
      <Button
        variant="ghost"
        size="sm"
        onclick={toggleMode}
        disabled={$isLoading}
        class="text-xs"
      >
        {#if currentMode === "pin"}
          <KeyRound class="mr-1.5 h-3 w-3" />
          Use Password
        {:else}
          <Hash class="mr-1.5 h-3 w-3" />
          Use PIN
        {/if}
      </Button>
    </div>
  </Card>
</div>
