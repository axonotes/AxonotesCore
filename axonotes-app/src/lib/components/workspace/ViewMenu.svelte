<script lang="ts">
  import {Menu, Sun, Moon, LockKeyhole} from "@lucide/svelte";
  import {Button} from "$lib/components/ui/button";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
  import {PANELS, addPanel, type PanelDefinition} from "$lib/stores/panels";
  import {toggleMode, mode} from "mode-watcher";
  import {app, databaseMode} from "$lib/stores/app";
  import * as m from "$lib/paraglide/messages.js";

  function handleAddPanel(panel: PanelDefinition) {
    addPanel(panel.id);
  }

  // Only show lock option if database has encryption enabled
  const showLockOption = $derived($databaseMode !== "none");
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger>
    {#snippet child({props})}
      <Button
        {...props}
        variant="ghost"
        size="icon"
        class="h-8 w-8"
        style="-webkit-app-region: no-drag"
      >
        <Menu class="h-4 w-4" />
        <span class="sr-only">View menu</span>
      </Button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content align="start" class="w-48">
    <DropdownMenu.Label>Panels</DropdownMenu.Label>
    <DropdownMenu.Separator />
    {#each PANELS as panel (panel.id)}
      {@const Icon = panel.icon}
      <DropdownMenu.Item onclick={() => handleAddPanel(panel)}>
        <Icon class="mr-2 h-4 w-4" />
        {panel.name}
      </DropdownMenu.Item>
    {/each}

    <DropdownMenu.Separator />

    <DropdownMenu.Item onclick={toggleMode}>
      {#if mode.current === "dark"}
        <Sun class="mr-2 h-4 w-4" />
        Light mode
      {:else}
        <Moon class="mr-2 h-4 w-4" />
        Dark mode
      {/if}
    </DropdownMenu.Item>

    {#if showLockOption}
      <DropdownMenu.Item onclick={() => app.lockDatabase()}>
        <LockKeyhole class="mr-2 h-4 w-4" />
        {m.common_action_lock_database()}
      </DropdownMenu.Item>
    {/if}
  </DropdownMenu.Content>
</DropdownMenu.Root>
