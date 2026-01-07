<script lang="ts">
  import {Shield, User, Globe, HardDrive, Info} from "@lucide/svelte";
  import * as m from "$lib/paraglide/messages.js";

  // Section components
  import SettingsSecurity from "./settings/SettingsSecurity.svelte";
  import SettingsAccount from "./settings/SettingsAccount.svelte";
  import SettingsLanguage from "./settings/SettingsLanguage.svelte";
  import SettingsStorage from "./settings/SettingsStorage.svelte";
  import SettingsAbout from "./settings/SettingsAbout.svelte";

  interface Props {
    panelId?: string;
    params?: Record<string, unknown>;
  }

  let {panelId, params}: Props = $props();

  type Section = "security" | "account" | "language" | "storage" | "about";

  let activeSection = $state<Section>("security");

  const sections: {id: Section; label: string; icon: typeof Shield}[] = [
    {id: "security", label: m.settings_nav_security(), icon: Shield},
    {id: "account", label: m.settings_nav_account(), icon: User},
    {id: "language", label: m.settings_nav_language(), icon: Globe},
    {id: "storage", label: m.settings_nav_storage(), icon: HardDrive},
    {id: "about", label: m.settings_nav_about(), icon: Info},
  ];
</script>

<div class="flex h-full">
  <!-- Left Navigation -->
  <nav class="w-48 shrink-0 border-r p-4">
    <ul class="space-y-1">
      {#each sections as section (section.id)}
        {@const Icon = section.icon}
        <li>
          <button
            onclick={() => (activeSection = section.id)}
            class="flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors {activeSection ===
            section.id
              ? 'bg-accent text-accent-foreground font-medium'
              : 'text-muted-foreground hover:bg-accent/50 hover:text-foreground'}"
          >
            <Icon class="h-4 w-4" />
            {section.label}
          </button>
        </li>
      {/each}
    </ul>
  </nav>

  <!-- Content Area -->
  <div class="flex-1 overflow-auto">
    {#if activeSection === "security"}
      <SettingsSecurity />
    {:else if activeSection === "account"}
      <SettingsAccount />
    {:else if activeSection === "language"}
      <SettingsLanguage />
    {:else if activeSection === "storage"}
      <SettingsStorage />
    {:else if activeSection === "about"}
      <SettingsAbout />
    {/if}
  </div>
</div>
