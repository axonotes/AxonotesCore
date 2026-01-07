<script lang="ts">
  interface Props {
    panelId?: string;
    params?: Record<string, unknown>;
  }

  let {panelId, params}: Props = $props();

  // Get file info from params if provided
  const fileName = $derived((params?.fileName as string) ?? "Untitled");
</script>

<div class="flex h-full flex-col">
  <div class="border-b p-2">
    <h3 class="text-sm font-medium">{fileName}</h3>
  </div>
  <div class="flex-1 overflow-auto p-4">
    <textarea
      class="bg-background text-foreground h-full w-full resize-none border-none outline-none"
      placeholder="Start writing..."
    ></textarea>
  </div>
</div>
