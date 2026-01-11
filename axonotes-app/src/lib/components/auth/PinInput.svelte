<script lang="ts">
  import {Label} from "$lib/components/ui/label";
  import {Eye, EyeOff} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  interface Props {
    id: string;
    label?: string;
    value: string;
    disabled?: boolean;
    autofocus?: boolean;
    onchange?: (value: string) => void;
    onenter?: () => void;
    oncomplete?: () => void;
    onbackspaceatstart?: () => void;
  }

  let {
    id,
    label,
    value = $bindable(""),
    disabled = false,
    autofocus = false,
    onchange,
    onenter,
    oncomplete,
    onbackspaceatstart,
  }: Props = $props();

  const PIN_LENGTH = 6;

  let digits = $state<string[]>(Array(PIN_LENGTH).fill(""));
  let inputs: HTMLInputElement[] = [];
  let showPin = $state(false);

  // Sync external value to internal digits
  $effect(() => {
    if (value !== digits.join("")) {
      const chars = value.toUpperCase().slice(0, PIN_LENGTH).split("");
      digits = [...chars, ...Array(PIN_LENGTH - chars.length).fill("")];
    }
  });

  function handleInput(index: number, event: Event) {
    const input = event.target as HTMLInputElement;
    const char = input.value.toUpperCase();

    // Only allow 0-9 and A-Z
    if (char && !/^[0-9A-Z]$/.test(char)) {
      input.value = showPin ? digits[index] : digits[index] ? "●" : "";
      return;
    }

    digits[index] = char;
    value = digits.join("");
    onchange?.(value);

    // Auto-advance to next input
    if (char && index < PIN_LENGTH - 1) {
      inputs[index + 1]?.focus();
    }

    // Call oncomplete when PIN is fully entered
    if (value.length === PIN_LENGTH) {
      oncomplete?.();
    }
  }

  function handleKeydown(index: number, event: KeyboardEvent) {
    // Handle Enter - submit if PIN is complete
    if (event.key === "Enter" && value.length === PIN_LENGTH) {
      event.preventDefault();
      onenter?.();
      return;
    }

    // Handle backspace
    if (event.key === "Backspace" && !digits[index]) {
      if (index > 0) {
        inputs[index - 1]?.focus();
      } else {
        // At first box with no value - go back to previous input
        onbackspaceatstart?.();
      }
    }

    // Handle arrow keys
    if (event.key === "ArrowLeft" && index > 0) {
      event.preventDefault();
      inputs[index - 1]?.focus();
    }
    if (event.key === "ArrowRight" && index < PIN_LENGTH - 1) {
      event.preventDefault();
      inputs[index + 1]?.focus();
    }
  }

  // Focus the first empty input (for programmatic focus)
  export function focus() {
    const emptyIndex = digits.findIndex((d) => !d);
    inputs[emptyIndex >= 0 ? emptyIndex : 0]?.focus();
  }

  // Focus the last filled input (for going back)
  export function focusLast() {
    const lastFilledIndex = digits.findLastIndex((d) => d !== "");
    inputs[lastFilledIndex >= 0 ? lastFilledIndex : PIN_LENGTH - 1]?.focus();
  }

  function handlePaste(event: ClipboardEvent) {
    event.preventDefault();
    const pastedData = event.clipboardData?.getData("text").toUpperCase() || "";
    const chars = pastedData.slice(0, PIN_LENGTH).split("");

    // Validate all characters
    if (chars.every((char) => /^[0-9A-Z]$/.test(char))) {
      chars.forEach((char, i) => {
        digits[i] = char;
      });

      // Fill remaining with empty
      for (let i = chars.length; i < PIN_LENGTH; i++) {
        digits[i] = "";
      }

      value = digits.join("");
      onchange?.(value);

      // Focus the next empty input or the last one
      const nextIndex = Math.min(chars.length, PIN_LENGTH - 1);
      inputs[nextIndex]?.focus();

      // Call oncomplete if PIN is fully entered
      if (value.length === PIN_LENGTH) {
        oncomplete?.();
      }
    }
  }

  function handleFocus(event: FocusEvent) {
    const input = event.target as HTMLInputElement;
    input.select();
  }

  function getDisplayValue(index: number): string {
    if (!digits[index]) return "";
    return showPin ? digits[index] : "●";
  }
</script>

<div class="space-y-2">
  {#if label}
    <Label for={id}>{label}</Label>
  {/if}
  <div class="flex items-center justify-center gap-2" onpaste={handlePaste}>
    {#each Array(PIN_LENGTH) as _, i (i)}
      <input
        bind:this={inputs[i]}
        id={i === 0 ? id : undefined}
        type="text"
        maxlength="1"
        inputmode="text"
        autocomplete="off"
        autocapitalize="characters"
        class="border-input bg-background ring-ring h-12 w-10 rounded-md border text-center font-mono text-lg uppercase transition-shadow focus:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
        value={getDisplayValue(i)}
        {disabled}
        autofocus={autofocus && i === 0}
        oninput={(e) => handleInput(i, e)}
        onkeydown={(e) => handleKeydown(i, e)}
        onfocus={handleFocus}
      />
    {/each}
    <button
      aria-label={showPin ? m.common_pin_hide() : m.common_pin_show()}
      class="text-muted-foreground hover:text-foreground ml-1 p-2 transition-colors"
      onclick={() => (showPin = !showPin)}
      tabindex={-1}
      type="button"
    >
      {#if showPin}
        <EyeOff class="h-4 w-4" />
      {:else}
        <Eye class="h-4 w-4" />
      {/if}
    </button>
  </div>
</div>
