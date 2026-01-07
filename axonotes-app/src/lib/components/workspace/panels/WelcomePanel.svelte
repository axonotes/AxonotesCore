<script lang="ts">
  import {Layers, GripVertical, Menu, Plus, LayoutGrid} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

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
        {m.welcome_title()}
      </h1>
      <p class="text-muted-foreground text-sm leading-normal">
        {m.welcome_description()}
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
          <p class="text-sm font-medium">{m.welcome_panels_title()}</p>
          <p class="text-muted-foreground text-sm leading-normal">
            {m.welcome_panels_description()}
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
          <p class="text-sm font-medium">{m.welcome_resize_title()}</p>
          <p class="text-muted-foreground text-sm leading-normal">
            {m.welcome_resize_description()}
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
          <p class="text-sm font-medium">{m.welcome_arrange_title()}</p>
          <p class="text-muted-foreground text-sm leading-normal">
            {m.welcome_arrange_description()}
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
          <p class="text-sm font-medium">{m.welcome_workspaces_title()}</p>
          <p class="text-muted-foreground text-sm leading-normal">
            {m.welcome_workspaces_description()}
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
          <p class="text-sm font-medium">{m.welcome_create_title()}</p>
          <p class="text-muted-foreground text-sm leading-normal">
            {m.welcome_create_description()}
          </p>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <p class="text-muted-foreground text-center text-xs">
      {m.welcome_footer()}
    </p>
  </div>
</div>
