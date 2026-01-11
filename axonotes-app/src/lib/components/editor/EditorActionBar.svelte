<script lang="ts">
  import {Share2} from "@lucide/svelte";
  import {Button} from "$lib/components/ui/button";
  import {ShareDialog} from "$lib/components/share";
  import * as m from "$lib/paraglide/messages.js";

  interface Props {
    docId: string;
    docName?: string;
  }

  let {docId, docName}: Props = $props();

  let shareDialogOpen = $state(false);
</script>

<!-- Floating action bar container - positioned at top of scrollable area -->
<div
  class="pointer-events-none sticky top-0 z-10 flex justify-center px-4 pt-3"
>
  <!-- The actual floating bar -->
  <div
    class="bg-background/90 pointer-events-auto flex w-full max-w-2xl items-center justify-between rounded-md border px-3 py-1.5 backdrop-blur-md"
    style="width: 80%;"
  >
    <!-- Left side - future formatting tools will go here -->
    <div class="flex items-center gap-1">
      <!-- Placeholder for formatting buttons -->
    </div>

    <!-- Right side - share and other actions -->
    <div class="flex items-center gap-1">
      <Button
        variant="ghost"
        size="sm"
        onclick={() => (shareDialogOpen = true)}
        class="h-8 gap-2 px-2"
      >
        <Share2 class="h-4 w-4" />
        <span class="hidden sm:inline">{m.share_context_menu_share()}</span>
      </Button>
    </div>
  </div>
</div>

<ShareDialog bind:open={shareDialogOpen} {docId} {docName} />
