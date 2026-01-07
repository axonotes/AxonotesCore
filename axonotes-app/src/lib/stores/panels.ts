/**
 * Panel Registry
 *
 * Central registry for all dockview panel types.
 * To add a new panel:
 * 1. Create a Svelte component
 * 2. Add it to PANELS array below
 * 3. Done!
 */

import type {Component} from "svelte";
import {writable, get} from "svelte/store";
import {Sidebar, FileText, Home, Settings} from "@lucide/svelte";
import type {DockviewApi, Parameters} from "dockview-core";

// Import panel components
import SidebarPanel from "$lib/components/workspace/panels/SidebarPanel.svelte";
import EditorPanel from "$lib/components/workspace/panels/EditorPanel.svelte";
import WelcomePanel from "$lib/components/workspace/panels/WelcomePanel.svelte";
import SettingsPanel from "$lib/components/workspace/panels/SettingsPanel.svelte";

/**
 * Panel definition for the registry
 */
export interface PanelDefinition {
  /** Unique identifier for this panel type */
  id: string;
  /** Display name shown in menus */
  name: string;
  /** Lucide icon component */
  icon: Component;
  /** The Svelte component to render */
  component: Component;
  /** If true, only one instance can be open at a time */
  singleton?: boolean;
  /** Default position when opening: 'left' | 'right' | 'bottom' | 'center' */
  defaultPosition?: "left" | "right" | "bottom" | "center";
}

/**
 * All registered panels.
 * Add new panels here - they'll automatically appear in ViewMenu.
 */
export const PANELS: PanelDefinition[] = [
  {
    id: "sidebar",
    name: "Explorer",
    icon: Sidebar,
    component: SidebarPanel,
    singleton: true,
    defaultPosition: "left",
  },
  {
    id: "editor",
    name: "Editor",
    icon: FileText,
    component: EditorPanel,
    defaultPosition: "center",
  },
  {
    id: "welcome",
    name: "Welcome",
    icon: Home,
    component: WelcomePanel,
    singleton: true,
    defaultPosition: "center",
  },
  {
    id: "settings",
    name: "Settings",
    icon: Settings,
    component: SettingsPanel,
    singleton: true,
    defaultPosition: "center",
  },
];

/**
 * Get a panel definition by ID
 */
export function getPanelById(id: string): PanelDefinition | undefined {
  return PANELS.find((p) => p.id === id);
}

/**
 * Build component registry for dockview from PANELS
 */
export function buildComponentRegistry(): Record<string, Component> {
  const registry: Record<string, Component> = {};
  for (const panel of PANELS) {
    registry[panel.id] = panel.component;
  }
  return registry;
}

/**
 * Dockview API store
 * Set by WorkspaceContainer, used by ViewMenu and other components
 */
const dockviewApiStore = writable<DockviewApi | null>(null);

export const dockviewApi = {
  subscribe: dockviewApiStore.subscribe,
  set: (api: DockviewApi | null) => dockviewApiStore.set(api),
};

/**
 * Add a panel using the global dockview API
 */
export function addPanel(
  panelType: string,
  options?: {id?: string; params?: Parameters; title?: string}
): void {
  const api = get(dockviewApiStore);

  if (!api) {
    console.warn("[Panels] No dockview API available");
    return;
  }

  const panelDef = getPanelById(panelType);
  if (!panelDef) {
    console.warn(`[Panels] Unknown panel type: ${panelType}`);
    return;
  }

  const id = options?.id ?? `${panelType}-${Date.now()}`;
  const params = options?.params ?? {};
  const title = options?.title ?? panelDef.name;

  // Check if panel with this exact ID already exists
  if (options?.id) {
    const existingById = api.getPanel(options.id);
    if (existingById) {
      // Focus existing panel instead of creating new one
      existingById.api.setActive();
      return;
    }
  }

  // Check singleton constraint
  if (panelDef.singleton) {
    const existing = api.panels.find((p) => {
      const componentType = (
        p as unknown as {view?: {contentComponent?: string}}
      ).view?.contentComponent;
      return componentType === panelType;
    });
    if (existing) {
      // Focus existing panel instead of creating new one
      existing.api.setActive();
      return;
    }
  }

  // For center panels (editors, settings, welcome), try to add to existing center group
  // or to the right of the sidebar if no center panel exists
  if (panelDef.defaultPosition === "center") {
    // Find an existing center panel (non-sidebar) to add as a tab
    const centerPanel = api.panels.find((p) => {
      const componentType = (
        p as unknown as {view?: {contentComponent?: string}}
      ).view?.contentComponent;
      return componentType !== "sidebar";
    });

    if (centerPanel) {
      // Add to the same group as the existing center panel
      api.addPanel({
        id,
        component: panelType,
        title,
        params,
        position: {referencePanel: centerPanel.id},
      });
      return;
    }

    // No center panel exists, add to the right of sidebar
    const sidebarPanel = api.panels.find((p) => {
      const componentType = (
        p as unknown as {view?: {contentComponent?: string}}
      ).view?.contentComponent;
      return componentType === "sidebar";
    });

    if (sidebarPanel) {
      api.addPanel({
        id,
        component: panelType,
        title,
        params,
        position: {referencePanel: sidebarPanel.id, direction: "right"},
      });
      return;
    }
  }

  // Map defaultPosition to dockview directions for non-center panels
  let direction: "left" | "right" | "above" | "below" | undefined;
  if (panelDef.defaultPosition === "left") direction = "left";
  else if (panelDef.defaultPosition === "right") direction = "right";
  else if (panelDef.defaultPosition === "bottom") direction = "below";

  api.addPanel({
    id,
    component: panelType,
    title,
    params,
    ...(direction && {position: {direction}}),
  });
}
