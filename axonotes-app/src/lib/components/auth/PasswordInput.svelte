<script lang="ts">
  import {Input} from "$lib/components/ui/input";
  import {Label} from "$lib/components/ui/label";
  import {Eye, EyeOff} from "@lucide/svelte";
  import zxcvbn from "zxcvbn";
  import * as m from "$lib/paraglide/messages.js";

  interface Props {
    id: string;
    label?: string;
    placeholder?: string;
    value: string;
    disabled?: boolean;
    showStrength?: boolean;
    autofocus?: boolean;
    onchange?: (value: string) => void;
  }

  let {
    id,
    label,
    placeholder = "",
    value = $bindable(""),
    disabled = false,
    showStrength = false,
    autofocus = false,
    onchange,
  }: Props = $props();

  let showPassword = $state(false);

  // Password strength calculation
  let strength = $derived.by(() => {
    if (!value || !showStrength) return null;
    return zxcvbn(value);
  });

  let strengthScore = $derived(strength?.score ?? 0);

  let strengthLabel = $derived.by(() => {
    if (!strength) return "";
    switch (strength.score) {
      case 0:
        return m.password_strength_weak();
      case 1:
        return m.password_strength_fair();
      case 2:
        return m.password_strength_good();
      case 3:
        return m.password_strength_strong();
      case 4:
        return m.password_strength_very_strong();
      default:
        return "";
    }
  });

  let strengthColor = $derived.by(() => {
    if (!strength) return "bg-muted";
    switch (strength.score) {
      case 0:
        return "bg-destructive";
      case 1:
        return "bg-destructive/70";
      case 2:
        return "bg-primary/50";
      case 3:
        return "bg-primary/80";
      case 4:
        return "bg-primary";
      default:
        return "bg-muted";
    }
  });

  let strengthTextColor = $derived.by(() => {
    if (!strength) return "text-muted-foreground";
    switch (strength.score) {
      case 0:
        return "text-destructive";
      case 1:
        return "text-destructive/80";
      case 2:
        return "text-muted-foreground";
      case 3:
        return "text-primary";
      case 4:
        return "text-primary";
      default:
        return "text-muted-foreground";
    }
  });

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    value = target.value;
    onchange?.(value);
  }
</script>

<div class="space-y-2">
  {#if label}
    <Label for={id}>{label}</Label>
  {/if}
  <div class="relative">
    <Input
      {autofocus}
      class="pr-10"
      {disabled}
      {id}
      oninput={handleInput}
      {placeholder}
      type={showPassword ? "text" : "password"}
      {value}
    />
    <button
      aria-label={showPassword
        ? m.common_password_hide()
        : m.common_password_show()}
      class="text-muted-foreground hover:text-foreground absolute top-1/2 right-3 -translate-y-1/2 transition-colors"
      onclick={() => (showPassword = !showPassword)}
      tabindex={-1}
      type="button"
    >
      {#if showPassword}
        <EyeOff class="h-4 w-4" />
      {:else}
        <Eye class="h-4 w-4" />
      {/if}
    </button>
  </div>

  {#if showStrength && value}
    <div class="space-y-1.5">
      <!-- Strength bar -->
      <div class="bg-muted h-1.5 w-full overflow-hidden rounded-full">
        <div
          class="{strengthColor} h-full transition-all duration-300"
          style="width: {(strengthScore + 1) * 20}%"
        ></div>
      </div>
      <!-- Strength label -->
      <p class="{strengthTextColor} text-sm font-medium">{strengthLabel}</p>
    </div>
  {/if}
</div>
