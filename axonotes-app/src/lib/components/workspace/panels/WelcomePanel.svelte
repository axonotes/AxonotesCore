<script lang="ts">
  import {Layers, GripVertical, Menu, Plus, LayoutGrid} from "@lucide/svelte";

  interface Props {
    panelId?: string;
    params?: Record<string, unknown>;
  }

  let {panelId, params}: Props = $props();
</script>

<div class="flex h-full w-full items-center justify-center p-8">
  <div class="w-full max-w-md space-y-8">
    <!-- Header -->
    <div class="space-y-4 text-center">
      <h1 class="text-3xl leading-tight font-semibold tracking-tight">
        Welcome to Axonotes
      </h1>
      <p class="text-muted-foreground text-sm leading-normal">
        Your workspace is ready. Here's how to get started:
      </p>
    </div>

    <!-- Instructions -->
    <div class="space-y-5 text-left">
      <div class="flex items-start gap-3">
        <div
          class="bg-muted flex h-8 w-8 shrink-0 items-center justify-center rounded-md"
        >
          <Menu class="h-4 w-4" />
        </div>
        <div class="space-y-1">
          <p class="text-sm font-medium">Open panels</p>
          <p class="text-muted-foreground text-sm leading-normal">
            Use the menu in the top-left corner to add new panels
          </p>
        </div>
      </div>

      <div class="flex items-start gap-3">
        <div
          class="bg-muted flex h-8 w-8 shrink-0 items-center justify-center rounded-md"
        >
          <GripVertical class="h-4 w-4" />
        </div>
        <div class="space-y-1">
          <p class="text-sm font-medium">Resize panels</p>
          <p class="text-muted-foreground text-sm leading-normal">
            Drag the borders between panels to resize them
          </p>
        </div>
      </div>

      <div class="flex items-start gap-3">
        <div
          class="bg-muted flex h-8 w-8 shrink-0 items-center justify-center rounded-md"
        >
          <Layers class="h-4 w-4" />
        </div>
        <div class="space-y-1">
          <p class="text-sm font-medium">Arrange your workspace</p>
          <p class="text-muted-foreground text-sm leading-normal">
            Drag tabs to rearrange or split into new groups
          </p>
        </div>
      </div>

      <div class="flex items-start gap-3">
        <div
          class="bg-muted flex h-8 w-8 shrink-0 items-center justify-center rounded-md"
        >
          <LayoutGrid class="h-4 w-4" />
        </div>
        <div class="space-y-1">
          <p class="text-sm font-medium">Workspaces</p>
          <p class="text-muted-foreground text-sm leading-normal">
            Each workspace has its own panel layout. Use the workspace bar at
            the bottom to switch between them or create new ones.
          </p>
        </div>
      </div>

      <div class="flex items-start gap-3">
        <div
          class="bg-muted flex h-8 w-8 shrink-0 items-center justify-center rounded-md"
        >
          <Plus class="h-4 w-4" />
        </div>
        <div class="space-y-1">
          <p class="text-sm font-medium">Create workspace</p>
          <p class="text-muted-foreground text-sm leading-normal">
            Click the + button in the workspace bar to add a new workspace.
            Right-click a workspace tab to rename or delete it.
          </p>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <p class="text-muted-foreground text-center text-xs">
      Your layout saves automatically per workspace
    </p>
  </div>
</div>
