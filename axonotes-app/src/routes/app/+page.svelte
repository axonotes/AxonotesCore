<script lang="ts">
  import {goto} from "$app/navigation";
  import {resolve} from "$app/paths";
  import {onMount, onDestroy} from "svelte";
  import {isReady} from "$lib/stores/app";
  import {
    workspaceStore,
    activeWorkspace,
    isLoading,
  } from "$lib/stores/workspace";
  import WorkspaceContainer from "$lib/components/workspace/WorkspaceContainer.svelte";
  import WorkspaceIndicator from "$lib/components/workspace/WorkspaceIndicator.svelte";
  import {Spinner} from "$lib/components/ui/spinner";
  import * as m from "$lib/paraglide/messages.js";

  let workspaceContainer: WorkspaceContainer;

  // Redirect to login if not ready
  onMount(async () => {
    if (!$isReady) {
      goto(resolve("/"), {replaceState: true});
      return;
    }

    // Load workspaces
    await workspaceStore.loadWorkspaces();

    // Set up keyboard shortcuts for workspace switching
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
  });

  function handleKeydown(event: KeyboardEvent) {
    // Ctrl/Cmd + number to switch workspaces
    if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey) {
      const num = parseInt(event.key, 10);
      if (num >= 1 && num <= 9) {
        event.preventDefault();
        workspaceStore.switchToIndex(num);
        return;
      }
    }

    // Ctrl/Cmd + [ or ] to switch to previous/next workspace
    if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey) {
      if (event.key === "[") {
        event.preventDefault();
        workspaceStore.previousWorkspace();
        return;
      }
      if (event.key === "]") {
        event.preventDefault();
        workspaceStore.nextWorkspace();
        return;
      }
    }
  }

  function handleLayoutChange(config: string) {
    workspaceStore.saveLayout(config);
  }
</script>

{#if $isReady}
  <div class="bg-background relative h-full w-full">
    {#if $isLoading}
      <div class="flex h-full items-center justify-center">
        <Spinner size="lg" />
      </div>
    {:else if $activeWorkspace}
      <WorkspaceContainer
        bind:this={workspaceContainer}
        initialConfig={$activeWorkspace.config}
        onLayoutChange={handleLayoutChange}
      />
    {:else}
      <div class="flex h-full items-center justify-center">
        <p class="text-muted-foreground text-sm">{m.app_no_workspace()}</p>
      </div>
    {/if}

    <!-- Workspace indicator at bottom -->
    <WorkspaceIndicator />
  </div>
{/if}
