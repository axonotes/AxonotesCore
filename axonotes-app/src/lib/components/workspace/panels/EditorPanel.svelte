<script lang="ts">
  interface Props {
    panelId?: string;
    params?: Record<string, unknown>;
  }

  let {panelId, params}: Props = $props();

  // Get document info from params
  const docId = $derived((params?.docId as string) ?? null);
  const fileName = $derived((params?.fileName as string) ?? "Untitled");
</script>

<div class="flex h-full flex-col">
  <div class="flex-1 overflow-auto p-4">
    <p class="text-muted-foreground text-center text-sm">
      Editor content for document will appear here.
    </p>
    {#if docId}
      <p class="text-muted-foreground mt-2 text-center font-mono text-xs">
        {docId}
      </p>
    {/if}
  </div>
</div>
