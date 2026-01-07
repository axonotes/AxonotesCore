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
import {Sidebar, FileText, Home} from "@lucide/svelte";
import type {DockviewApi, Parameters} from "dockview-core";

// Import panel components
import SidebarPanel from "$lib/components/workspace/panels/SidebarPanel.svelte";
import EditorPanel from "$lib/components/workspace/panels/EditorPanel.svelte";
import WelcomePanel from "$lib/components/workspace/panels/WelcomePanel.svelte";

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
    name: "Sidebar",
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
  options?: {id?: string; params?: Parameters}
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

  // Map defaultPosition to dockview directions
  let direction: "left" | "right" | "above" | "below" | undefined;
  if (panelDef.defaultPosition === "left") direction = "left";
  else if (panelDef.defaultPosition === "right") direction = "right";
  else if (panelDef.defaultPosition === "bottom") direction = "below";

  api.addPanel({
    id,
    component: panelType,
    title: panelDef.name,
    params,
    ...(direction && {position: {direction}}),
  });
}
