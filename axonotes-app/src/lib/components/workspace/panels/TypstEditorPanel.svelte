<script lang="ts">
  import * as m from "$lib/paraglide/messages.js";
  import {Loader2, AlertCircle} from "@lucide/svelte";
  import {Button} from "$lib/components/ui/button";

  interface Props {
    panelId?: string;
    params?: Record<string, unknown>;
  }

  let {panelId, params}: Props = $props();

  const docId = $derived((params?.docId as string) ?? null);
  const fileName = $derived((params?.fileName as string) ?? "Untitled");

  let loading = $state(false);
  let error = $state<string | null>(null);
</script>

<div class="flex h-full flex-col">
  {#if !docId}
    <div class="flex flex-1 items-center justify-center">
      <p class="text-muted-foreground text-center text-sm">
        {m.editor_placeholder()}
      </p>
    </div>
  {:else if loading}
    <div class="flex flex-1 flex-col items-center justify-center gap-4">
      <Loader2 class="text-muted-foreground h-8 w-8 animate-spin" />
      <p class="text-muted-foreground text-sm">{m.editor_loading()}</p>
    </div>
  {:else if error}
    <div class="flex flex-1 flex-col items-center justify-center gap-4 p-6">
      <AlertCircle class="text-destructive h-12 w-12" />
      <div class="text-center">
        <h3 class="text-lg font-medium">{m.editor_error_load()}</h3>
        <p class="text-muted-foreground mt-1 text-sm">{error}</p>
      </div>
    </div>
  {:else}
    <div class="flex flex-1 flex-col items-center justify-center gap-4 p-6">
      <div class="text-center">
        <h3 class="text-foreground text-lg font-medium">{fileName}</h3>
        <p class="text-muted-foreground mt-2 text-sm">
          Typst editor — coming soon
        </p>
        <p class="text-muted-foreground mt-1 text-xs">
          Document ID: {docId}
        </p>
      </div>
    </div>
  {/if}
</div>
