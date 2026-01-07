<script lang="ts">
  import {Copy, Check} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  interface Props {
    words: string[];
    showCopyButton?: boolean;
  }

  let {words, showCopyButton = true}: Props = $props();

  let copied = $state(false);

  async function copyToClipboard() {
    try {
      await navigator.clipboard.writeText(words.join(" "));
      copied = true;
      setTimeout(() => {
        copied = false;
      }, 2000);
    } catch (err) {
      console.error("Failed to copy:", err);
    }
  }
</script>

<div class="space-y-4">
  <!-- 3x4 Grid of words -->
  <div class="bg-muted/50 grid grid-cols-3 gap-2 rounded-lg border p-4">
    {#each words as word, index (index)}
      <div
        class="bg-background flex items-center gap-2 rounded-md border px-3 py-2"
      >
        <span class="text-muted-foreground w-4 text-xs font-medium">
          {index + 1}
        </span>
        <span class="font-mono text-sm font-medium">
          {word}
        </span>
      </div>
    {/each}
  </div>

  {#if showCopyButton}
    <button
      type="button"
      onclick={copyToClipboard}
      class="text-muted-foreground hover:text-foreground mx-auto flex items-center gap-1.5 text-sm transition-colors"
    >
      {#if copied}
        <Check class="h-3.5 w-3.5" />
        <span>{m.common_action_copied()}</span>
      {:else}
        <Copy class="h-3.5 w-3.5" />
        <span>{m.common_action_copy()}</span>
      {/if}
    </button>
  {/if}
</div>
