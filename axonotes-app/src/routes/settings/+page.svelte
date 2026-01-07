<script lang="ts">
  import {DatabaseService} from "$lib/services/database";
  import {databaseMode} from "$lib/stores/app";
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
  import {AlertCircle, Lock, Trash2, RefreshCw} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

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
        error = m.settings_encryption_error_empty();
        return;
      }

      if (password !== confirmPassword) {
        error = m.settings_encryption_error_mismatch();
        return;
      }

      if (
        mode === "pin" &&
        (password.length !== 6 || !/^[0-9A-Z]+$/.test(password))
      ) {
        error = m.settings_encryption_error_pin_format();
        return;
      }

      await DatabaseService.setEncryption(password, mode);
      success =
        $databaseMode === "none"
          ? m.settings_encryption_success_set({mode})
          : m.settings_encryption_success_change({mode});
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
    if (!confirm(m.settings_danger_remove_confirm())) {
      return;
    }

    loading = true;
    error = "";
    success = "";

    try {
      await DatabaseService.removeEncryption();
      success = m.settings_encryption_success_remove();

      // Reload to re-initialize
      setTimeout(() => window.location.reload(), 1500);
    } catch (err: any) {
      error = err.toString();
    } finally {
      loading = false;
    }
  }

  async function handleSwitchMode() {
    loading = true;
    error = "";
    success = "";

    try {
      const newMode: "pin" | "pass" = $databaseMode === "pin" ? "pass" : "pin";
      await DatabaseService.switchMode(newMode);
      success = m.settings_switch_success({mode: newMode.toUpperCase()});

      // Reload to re-initialize
      setTimeout(() => window.location.reload(), 1500);
    } catch (err: any) {
      error = err.toString();
    } finally {
      loading = false;
    }
  }

  async function handleWipe() {
    if (!confirm(m.settings_danger_wipe_confirm())) {
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
    <h2 class="text-3xl font-semibold tracking-tight">{m.settings_title()}</h2>
    <p class="text-muted-foreground">{m.settings_description()}</p>
  </div>

  <div class="grid gap-4 lg:grid-cols-2">
    <!-- Current Status -->
    <Card>
      <CardHeader>
        <CardTitle>{m.settings_security_title()}</CardTitle>
        <CardDescription>{m.settings_security_description()}</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Lock class="h-4 w-4" />
            <span class="text-sm font-medium"
              >{m.settings_security_mode_label()}</span
            >
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
          <div
            class="flex items-start gap-2 rounded-md border border-yellow-500/20 bg-yellow-500/10 p-3"
          >
            <AlertCircle
              class="mt-0.5 h-4 w-4 text-yellow-700 dark:text-yellow-400"
            />
            <p class="text-sm text-yellow-700 dark:text-yellow-400">
              {m.settings_security_unencrypted_warning()}
            </p>
          </div>
        {/if}
      </CardContent>
    </Card>

    <!-- Set/Change Encryption -->
    <Card>
      <CardHeader>
        <CardTitle>
          {$databaseMode === "none"
            ? m.settings_encryption_title_set()
            : m.settings_encryption_title_change()}
        </CardTitle>
        <CardDescription>
          {$databaseMode === "none"
            ? m.settings_encryption_description_set()
            : m.settings_encryption_description_change()}
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="space-y-2">
          <div class="flex gap-4">
            <label class="flex cursor-pointer items-center gap-2">
              <input type="radio" bind:group={mode} value="pin" />
              <span class="text-sm">{m.settings_encryption_mode_pin()}</span>
            </label>
            <label class="flex cursor-pointer items-center gap-2">
              <input type="radio" bind:group={mode} value="pass" />
              <span class="text-sm"
                >{m.settings_encryption_mode_password()}</span
              >
            </label>
          </div>
        </div>

        <div class="space-y-2">
          <Label for="new-password">
            {mode === "pin"
              ? m.settings_encryption_new_label_pin()
              : m.settings_encryption_new_label_password()}
          </Label>
          <Input
            id="new-password"
            type="password"
            placeholder={mode === "pin"
              ? m.settings_encryption_new_placeholder_pin()
              : m.settings_encryption_new_placeholder_password()}
            bind:value={password}
            disabled={loading}
          />
        </div>

        <div class="space-y-2">
          <Label for="confirm-password">
            {mode === "pin"
              ? m.settings_encryption_confirm_label_pin()
              : m.settings_encryption_confirm_label_password()}
          </Label>
          <Input
            id="confirm-password"
            type="password"
            placeholder={m.auth_setup_master_confirm_placeholder()}
            bind:value={confirmPassword}
            disabled={loading}
          />
        </div>

        {#if error}
          <p class="text-destructive text-sm">{error}</p>
        {/if}

        {#if success}
          <p class="text-sm text-green-600 dark:text-green-400">{success}</p>
        {/if}

        <div class="flex gap-2">
          <Button onclick={handleSetEncryption} disabled={loading}>
            {$databaseMode === "none"
              ? m.settings_encryption_button_set()
              : m.settings_encryption_button_change()}
          </Button>

          {#if $databaseMode !== "none"}
            <Button
              variant="outline"
              onclick={handleRemoveEncryption}
              disabled={loading}
            >
              {m.settings_encryption_button_remove()}
            </Button>
          {/if}
        </div>
      </CardContent>
    </Card>

    <!-- Switch Mode -->
    {#if $databaseMode === "pin" || $databaseMode === "pass"}
      <Card class="lg:col-span-2">
        <CardHeader>
          <CardTitle>{m.settings_switch_title()}</CardTitle>
          <CardDescription>
            {m.settings_switch_description()}
          </CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          <div class="flex items-center justify-between">
            <div class="space-y-1">
              <p class="text-sm font-medium">
                {m.settings_switch_current({mode: $databaseMode.toUpperCase()})}
              </p>
              <p class="text-muted-foreground text-xs">
                {$databaseMode === "pin"
                  ? m.settings_switch_hint_pin()
                  : m.settings_switch_hint_password()}
              </p>
            </div>
            <Button
              variant="outline"
              onclick={handleSwitchMode}
              disabled={loading}
            >
              <RefreshCw class="mr-2 h-4 w-4" />
              {$databaseMode === "pin"
                ? m.settings_switch_to_password()
                : m.settings_switch_to_pin()}
            </Button>
          </div>

          {#if error}
            <p class="text-destructive text-sm">{error}</p>
          {/if}

          {#if success}
            <p class="text-sm text-green-600 dark:text-green-400">{success}</p>
          {/if}
        </CardContent>
      </Card>
    {/if}

    <!-- Danger Zone -->
    <Card class="border-destructive/50 lg:col-span-2">
      <CardHeader>
        <CardTitle class="text-destructive"
          >{m.settings_danger_title()}</CardTitle
        >
        <CardDescription>{m.settings_danger_description()}</CardDescription>
      </CardHeader>
      <CardContent>
        <Button variant="destructive" onclick={handleWipe} disabled={loading}>
          <Trash2 class="mr-2 h-4 w-4" />
          {m.settings_danger_wipe_button()}
        </Button>
        <p class="text-muted-foreground mt-2 text-xs">
          {m.settings_danger_wipe_description()}
        </p>
      </CardContent>
    </Card>
  </div>
</div>
