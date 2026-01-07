<script lang="ts">
  import * as m from "$lib/paraglide/messages.js";

  interface Props {
    /** Panel title to display */
    title?: string;
    /** Panel ID from dockview */
    panelId?: string;
  }

  let {title = m.workspace_panel_placeholder_title(), panelId = ""}: Props =
    $props();
</script>

<div
  class="bg-background flex h-full w-full flex-col items-center justify-center"
>
  <div class="text-muted-foreground space-y-2 text-center">
    <p class="text-sm font-medium">{title}</p>
    {#if panelId}
      <p class="text-muted-foreground/60 text-xs">ID: {panelId}</p>
    {/if}
  </div>
</div>
