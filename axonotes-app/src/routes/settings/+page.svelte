<script lang="ts">
  import {DatabaseService} from "$lib/services/database";
  import {databaseMode} from "$lib/stores/app";
  import {Button} from "$lib/components/ui/button";
  import {Card, CardContent, CardDescription, CardHeader, CardTitle} from "$lib/components/ui/card";
  import {Input} from "$lib/components/ui/input";
  import {Label} from "$lib/components/ui/label";
  import {Badge} from "$lib/components/ui/badge";
  import {AlertCircle, Lock, Trash2} from "@lucide/svelte";

  let password = $state("");
  let confirmPassword = $state("");
  let mode = $state<"pin" | "pass">("pass");
  let error = $state("");
  let success = $state("");
  let loading = $state(false);

  async function handleSetEncryption() {
    loading = true;
    error = "";
    success = "";

    try {
      if (!password) {
        error = "Please enter a password";
        return;
      }

      if (password !== confirmPassword) {
        error = "Passwords do not match";
        return;
      }

      if (mode === "pin" && (password.length !== 8 || !/^[0-9A-Z]+$/.test(password))) {
        error = "PIN must be exactly 8 characters (0-9, A-Z uppercase only)";
        return;
      }

      await DatabaseService.setEncryption(password, mode);
      success = `Encryption set! Mode: ${mode}. Reloading...`;
      password = "";
      confirmPassword = "";

      // Reload to re-initialize with new encryption
      setTimeout(() => window.location.reload(), 1500);
    } catch (err: any) {
      error = err.toString();
    } finally {
      loading = false;
    }
  }

  async function handleRemoveEncryption() {
    if (!confirm("Remove database encryption? This will make your data unencrypted.")) {
      return;
    }

    loading = true;
    error = "";
    success = "";

    try {
      await DatabaseService.removeEncryption();
      success = "Encryption removed! Reloading...";

      // Reload to re-initialize
      setTimeout(() => window.location.reload(), 1500);
    } catch (err: any) {
      error = err.toString();
    } finally {
      loading = false;
    }
  }

  async function handleWipe() {
    if (!confirm("Wipe entire database? This cannot be undone!")) {
      return;
    }

    loading = true;

    try {
      await DatabaseService.wipe();
      window.location.href = "/unlock";
    } catch (err: any) {
      error = err.toString();
      loading = false;
    }
  }
</script>

<div class="space-y-6">
  <div>
    <h2 class="text-3xl font-bold tracking-tight">Settings</h2>
    <p class="text-muted-foreground">Manage database security</p>
  </div>

  <div class="grid gap-4 lg:grid-cols-2">
    <!-- Current Status -->
    <Card>
      <CardHeader>
        <CardTitle>Database Security</CardTitle>
        <CardDescription>Current encryption status</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Lock class="h-4 w-4" />
            <span class="text-sm font-medium">Unlock Mode</span>
          </div>
          <Badge
            variant={$databaseMode === "none" ? "outline" : "default"}
            class={$databaseMode === "none"
              ? "border-yellow-500/20 bg-yellow-500/10 text-yellow-700 dark:text-yellow-400"
              : "border-green-500/20 bg-green-500/10 text-green-700 dark:text-green-400"}
          >
            {$databaseMode}
          </Badge>
        </div>

        {#if $databaseMode === "none"}
          <div class="flex items-start gap-2 rounded-md border border-yellow-500/20 bg-yellow-500/10 p-3">
            <AlertCircle class="mt-0.5 h-4 w-4 text-yellow-700 dark:text-yellow-400" />
            <p class="text-sm text-yellow-700 dark:text-yellow-400">
              Your database is not encrypted. Set a password for better security.
            </p>
          </div>
        {/if}
      </CardContent>
    </Card>

    <!-- Set/Change Encryption -->
    <Card>
      <CardHeader>
        <CardTitle>{$databaseMode === "none" ? "Set Encryption" : "Change Password"}</CardTitle>
        <CardDescription>
          {$databaseMode === "none" ? "Encrypt your database" : "Update encryption password"}
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="space-y-2">
          <div class="flex gap-4">
            <label class="flex cursor-pointer items-center gap-2">
              <input type="radio" bind:group={mode} value="pin" />
              <span class="text-sm">PIN (8 chars, 0-9A-Z)</span>
            </label>
            <label class="flex cursor-pointer items-center gap-2">
              <input type="radio" bind:group={mode} value="pass" />
              <span class="text-sm">Password (any)</span>
            </label>
          </div>
        </div>

        <div class="space-y-2">
          <Label for="new-password">New {mode === "pin" ? "PIN" : "Password"}</Label>
          <Input
            id="new-password"
            type="password"
            placeholder={mode === "pin" ? "8 characters (0-9A-Z)" : "Enter password"}
            bind:value={password}
            disabled={loading}
          />
        </div>

        <div class="space-y-2">
          <Label for="confirm-password">Confirm {mode === "pin" ? "PIN" : "Password"}</Label>
          <Input
            id="confirm-password"
            type="password"
            placeholder="Confirm"
            bind:value={confirmPassword}
            disabled={loading}
          />
        </div>

        {#if error}
          <p class="text-sm text-destructive">{error}</p>
        {/if}

        {#if success}
          <p class="text-sm text-green-600 dark:text-green-400">{success}</p>
        {/if}

        <div class="flex gap-2">
          <Button onclick={handleSetEncryption} disabled={loading}>
            {$databaseMode === "none" ? "Set Encryption" : "Change Password"}
          </Button>

          {#if $databaseMode !== "none"}
            <Button variant="outline" onclick={handleRemoveEncryption} disabled={loading}>
              Remove Encryption
            </Button>
          {/if}
        </div>
      </CardContent>
    </Card>

    <!-- Danger Zone -->
    <Card class="border-destructive/50 lg:col-span-2">
      <CardHeader>
        <CardTitle class="text-destructive">Danger Zone</CardTitle>
        <CardDescription>Irreversible actions</CardDescription>
      </CardHeader>
      <CardContent>
        <Button variant="destructive" onclick={handleWipe} disabled={loading}>
          <Trash2 class="mr-2 h-4 w-4" />
          Wipe Database
        </Button>
        <p class="text-muted-foreground mt-2 text-xs">
          Permanently deletes all data and resets encryption.
        </p>
      </CardContent>
    </Card>
  </div>
</div>
