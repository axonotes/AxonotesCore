<script lang="ts">
  import {
    Menu,
    Sun,
    Moon,
    LockKeyhole,
    Settings,
    FilePlus,
  } from "@lucide/svelte";
  import {Button} from "$lib/components/ui/button";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
  import {PANELS, addPanel, type PanelDefinition} from "$lib/stores/panels";
  import {toggleMode, mode} from "mode-watcher";
  import {app, databaseMode} from "$lib/stores/app";
  import {createDocument} from "$lib/stores/documents";
  import * as m from "$lib/paraglide/messages.js";

  // Filter out settings and editor from main panels list
  // Editor is opened via "Create Document" or sidebar, not directly
  const contentPanels = PANELS.filter(
    (p) => p.id !== "settings" && p.id !== "editor"
  );

  // Get translated panel name
  function getPanelName(id: string): string {
    switch (id) {
      case "sidebar":
        return m.panel_explorer();
      case "editor":
        return m.panel_editor();
      case "welcome":
        return m.panel_welcome();
      case "settings":
        return m.panel_settings();
      default:
        return id;
    }
  }

  let creating = $state(false);

  async function handleCreateDocument() {
    if (creating) return;
    creating = true;
    try {
      const docId = await createDocument();
      addPanel("editor", {
        id: `editor-${docId}`,
        params: {docId, fileName: "Default.doc"},
        title: "Default.doc",
      });
    } catch (error) {
      console.error("[ViewMenu] Failed to create document:", error);
    } finally {
      creating = false;
    }
  }

  function handleAddPanel(panel: PanelDefinition) {
    addPanel(panel.id);
  }

  function openSettings() {
    addPanel("settings");
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
    <DropdownMenu.Label>{m.view_menu_panels()}</DropdownMenu.Label>
    <DropdownMenu.Separator />
    <DropdownMenu.Item onclick={handleCreateDocument} disabled={creating}>
      <FilePlus class="mr-2 h-4 w-4" />
      {m.view_menu_create_document()}
    </DropdownMenu.Item>

    {#each contentPanels as panel (panel.id)}
      {@const Icon = panel.icon}
      <DropdownMenu.Item onclick={() => handleAddPanel(panel)}>
        <Icon class="mr-2 h-4 w-4" />
        {getPanelName(panel.id)}
      </DropdownMenu.Item>
    {/each}

    <DropdownMenu.Separator />

    <DropdownMenu.Item onclick={openSettings}>
      <Settings class="mr-2 h-4 w-4" />
      {m.settings_title()}
    </DropdownMenu.Item>

    <DropdownMenu.Item onclick={toggleMode}>
      {#if mode.current === "dark"}
        <Sun class="mr-2 h-4 w-4" />
        {m.view_menu_light_mode()}
      {:else}
        <Moon class="mr-2 h-4 w-4" />
        {m.view_menu_dark_mode()}
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
