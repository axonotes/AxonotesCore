<script lang="ts">
  import {getCurrentWindow} from "@tauri-apps/api/window";
  import {Minus, Square, X} from "@lucide/svelte";
  import ViewMenu from "./workspace/ViewMenu.svelte";
  import {isReady} from "$lib/stores/app";
  import * as m from "$lib/paraglide/messages.js";

  const appWindow = getCurrentWindow();

  let isMaximized = $state(false);

  async function minimize() {
    await appWindow.minimize();
  }

  async function toggleMaximize() {
    await appWindow.toggleMaximize();
    isMaximized = await appWindow.isMaximized();
  }

  async function close() {
    await appWindow.close();
  }

  // Check initial maximized state
  $effect(() => {
    appWindow.isMaximized().then((max) => (isMaximized = max));
  });
</script>

<div
  data-tauri-drag-region
  class="bg-background flex h-12 items-center justify-between border-b px-4 select-none"
>
  <div class="flex items-center gap-2">
    {#if $isReady}
      <ViewMenu />
    {/if}
    <span class="text-sm font-semibold">{m.common_app_name()}</span>
  </div>

  <div class="flex items-center gap-1">
    <button
      onclick={minimize}
      class="hover:bg-accent hover:text-accent-foreground flex h-8 w-8 items-center justify-center rounded-md transition-colors duration-150"
      style="-webkit-app-region: no-drag"
      aria-label="Minimize"
    >
      <Minus class="h-4 w-4" />
    </button>

    <button
      onclick={toggleMaximize}
      class="hover:bg-accent hover:text-accent-foreground flex h-8 w-8 items-center justify-center rounded-md transition-colors duration-150"
      style="-webkit-app-region: no-drag"
      aria-label={isMaximized ? "Restore" : "Maximize"}
    >
      <Square class="h-3.5 w-3.5" />
    </button>

    <button
      onclick={close}
      class="hover:bg-destructive hover:text-destructive-foreground flex h-8 w-8 items-center justify-center rounded-md transition-colors duration-150"
      style="-webkit-app-region: no-drag"
      aria-label="Close"
    >
      <X class="h-4 w-4" />
    </button>
  </div>
</div>
