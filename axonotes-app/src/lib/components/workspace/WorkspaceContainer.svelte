<script lang="ts">
  import {onMount, onDestroy, mount, unmount} from "svelte";
  import {SvelteMap} from "svelte/reactivity";
  import {
    DockviewComponent,
    type DockviewApi,
    type IContentRenderer,
    type Parameters,
    type AddPanelOptions,
    type DockviewTheme,
    type SerializedDockview,
  } from "dockview-core";
  import "dockview-core/dist/styles/dockview.css";
  import {activeWorkspace} from "$lib/stores/workspace";
  import {
    buildComponentRegistry,
    dockviewApi as dockviewApiStore,
  } from "$lib/stores/panels";

  // Custom theme that inherits from shadcn CSS variables
  const axonotesTheme: DockviewTheme = {
    name: "axonotes",
    className: "axonotes-dockview-theme",
  };

  interface Props {
    /** Initial layout configuration (JSON string) */
    initialConfig?: string;
    /** Callback when layout changes */
    onLayoutChange?: (config: string) => void;
  }

  let {initialConfig, onLayoutChange}: Props = $props();

  let containerEl: HTMLDivElement | undefined = $state();
  let dockview: DockviewComponent | null = null;
  let dockviewApi: DockviewApi | null = $state(null);
  let isInitialized = $state(false);

  // Store params for each panel since CreateComponentOptions doesn't include params
  const panelParamsMap = new SvelteMap<string, Parameters>();

  // Build component registry from PANELS
  const componentRegistry = buildComponentRegistry();

  // Sync dockview API to store when it changes
  $effect(() => {
    dockviewApiStore.set(dockviewApi);
  });

  /**
   * Custom Svelte content renderer for dockview.
   * Mounts Svelte 5 components into dockview panels.
   */
  class SvelteContentRenderer implements IContentRenderer {
    private _element: HTMLElement;
    private _component: ReturnType<typeof mount> | null = null;
    private _componentType: string;
    private _panelId: string;

    constructor(panelId: string, componentType: string) {
      this._element = document.createElement("div");
      this._element.className =
        "dockview-svelte-panel h-full w-full overflow-auto";
      this._componentType = componentType;
      this._panelId = panelId;
    }

    get element(): HTMLElement {
      return this._element;
    }

    init(parameters: {params: Parameters}): void {
      const Component = componentRegistry[this._componentType];
      if (!Component) {
        console.warn(
          `[Dockview] Unknown component type: ${this._componentType}`
        );
        return;
      }

      // Get stored params and merge with init params
      const storedParams = panelParamsMap.get(this._panelId) ?? {};
      const props = {
        ...storedParams,
        ...parameters.params,
        panelId: this._panelId,
      };

      this._component = mount(Component, {
        target: this._element,
        props,
      });
    }

    update(event: {params: Parameters}): void {
      // Svelte 5 reactivity handles updates via $state
      if (this._component && event.params) {
        // Could implement prop updates here if needed
      }
    }

    dispose(): void {
      if (this._component) {
        unmount(this._component);
        this._component = null;
      }
      panelParamsMap.delete(this._panelId);
    }
  }

  /**
   * Handle middle-click on tabs to close panels.
   */
  function handleMiddleClick(event: MouseEvent) {
    // Middle mouse button is button 1
    if (event.button !== 1) return;

    // Find if we clicked on a tab
    const target = event.target as HTMLElement;
    const tab = target.closest(".dv-tab") as HTMLElement | null;
    if (!tab || !dockviewApi) return;

    // Get panel ID from the tab's data attribute
    const panelId = tab.getAttribute("data-panel-id");
    if (!panelId) return;

    // Close the panel
    const panel = dockviewApi.getPanel(panelId);
    if (panel) {
      event.preventDefault();
      panel.api.close();
    }
  }

  /**
   * Initialize dockview with the given configuration.
   */
  function initializeDockview() {
    if (!containerEl || dockview) return;

    // Create dockview instance with custom theme
    dockview = new DockviewComponent(containerEl, {
      theme: axonotesTheme,
      createComponent: (options) => {
        return new SvelteContentRenderer(options.id, options.name);
      },
      disableFloatingGroups: true,
    });

    // Listen for layout changes (includes resize)
    dockview.onDidLayoutChange(() => {
      if (dockviewApi && isInitialized) {
        const config = serializeLayout();
        if (config && onLayoutChange) {
          onLayoutChange(config);
        }
      }
    });

    dockviewApi = dockview.api;

    // Add middle-click to close tabs
    containerEl.addEventListener("auxclick", handleMiddleClick);

    // Load initial layout or create default
    if (initialConfig) {
      try {
        loadLayout(initialConfig);
      } catch (error) {
        console.error("[Dockview] Failed to load layout:", error);
        createDefaultLayout();
      }
    } else {
      createDefaultLayout();
    }

    isInitialized = true;
  }

  /**
   * Load a layout from JSON configuration.
   * Uses dockview's built-in fromJSON for full state restoration including sizes.
   */
  function loadLayout(configJson: string) {
    if (!dockview) return;

    try {
      const config = JSON.parse(configJson);
      panelParamsMap.clear();

      // Check if this is a full dockview serialized layout (has grid property)
      if (config.grid && config.panels) {
        // Store params for each panel before loading
        for (const [panelId, panelData] of Object.entries(config.panels)) {
          const data = panelData as {params?: Parameters};
          if (data.params) {
            panelParamsMap.set(panelId, data.params);
          }
        }

        // Use dockview's built-in fromJSON for full restoration
        dockview.fromJSON(config as SerializedDockview);
      } else {
        // Legacy format or invalid - create default
        createDefaultLayout();
      }
    } catch (error) {
      console.error("[Dockview] Failed to parse layout:", error);
      createDefaultLayout();
    }
  }

  /**
   * Create the default sidebar + editor layout.
   */
  function createDefaultLayout() {
    if (!dockviewApi) return;

    dockviewApi.clear();
    panelParamsMap.clear();

    // Add sidebar panel (will be ~30% after main panel is added)
    panelParamsMap.set("sidebar", {});
    dockviewApi.addPanel({
      id: "sidebar",
      component: "sidebar",
      title: "Files",
      params: {},
    });

    // Add main welcome panel
    panelParamsMap.set("main", {});
    dockviewApi.addPanel({
      id: "main",
      component: "welcome",
      title: "Welcome",
      params: {},
      position: {referencePanel: "sidebar", direction: "right"},
    });
  }

  /**
   * Serialize the current layout to JSON.
   * Uses dockview's built-in toJSON for full state including panel sizes.
   */
  function serializeLayout(): string | null {
    if (!dockview) return null;

    try {
      // Get full serialized state from dockview (includes grid sizes)
      const serialized = dockview.toJSON();

      // Get workspace name from current config
      let name = "Workspace";
      if (initialConfig) {
        try {
          const currentConfig = JSON.parse(initialConfig);
          name = currentConfig.name ?? name;
        } catch {
          // Ignore parse errors
        }
      }

      // Add name to the serialized config
      return JSON.stringify({
        ...serialized,
        name,
      });
    } catch (error) {
      console.error("[Dockview] Failed to serialize layout:", error);
      return null;
    }
  }

  /**
   * Cleanup dockview instance.
   */
  function cleanup() {
    if (containerEl) {
      containerEl.removeEventListener("auxclick", handleMiddleClick);
    }
    if (dockview) {
      dockview.dispose();
      dockview = null;
      dockviewApi = null;
      isInitialized = false;
      panelParamsMap.clear();
    }
  }

  /**
   * Add a new panel to the layout.
   */
  export function addPanel(options: {
    id: string;
    component: string;
    params?: Parameters;
    position?: {
      referencePanel?: string;
      direction?: "left" | "right" | "above" | "below" | "within";
    };
  }) {
    if (!dockviewApi) return null;

    // Store params
    if (options.params) {
      panelParamsMap.set(options.id, options.params);
    }

    const addOptions: AddPanelOptions = {
      id: options.id,
      component: options.component,
      params: options.params ?? {},
    };

    if (options.position?.referencePanel) {
      addOptions.position = {
        referencePanel: options.position.referencePanel,
        direction: options.position.direction,
      };
    }

    return dockviewApi.addPanel(addOptions);
  }

  /**
   * Remove a panel from the layout.
   */
  export function removePanel(id: string) {
    if (!dockviewApi) return;

    const panel = dockviewApi.getPanel(id);
    if (panel) {
      panel.api.close();
    }
  }

  /**
   * Get the dockview API for external access.
   */
  export function getApi(): DockviewApi | null {
    return dockviewApi;
  }

  // Reload layout when active workspace changes
  let previousWorkspaceId: string | null = null;
  $effect(() => {
    const workspace = $activeWorkspace;
    if (workspace && dockviewApi && isInitialized) {
      // Only reload if workspace actually changed
      if (previousWorkspaceId !== workspace.id) {
        previousWorkspaceId = workspace.id;
        loadLayout(workspace.config);
      }
    }
  });

  onMount(() => {
    // Wait for next tick to ensure containerEl is bound
    requestAnimationFrame(() => {
      initializeDockview();
    });
  });

  onDestroy(() => {
    cleanup();
  });
</script>

<div bind:this={containerEl} class="h-full w-full"></div>

<style>
  /* Custom Axonotes theme - inherits from shadcn CSS variables */
  :global(.axonotes-dockview-theme) {
    /* Core backgrounds */
    --dv-group-view-background-color: var(--background);
    --dv-tabs-and-actions-container-background-color: var(--background);

    /* Active group tabs */
    --dv-activegroup-visiblepanel-tab-background-color: var(--background);
    --dv-activegroup-hiddenpanel-tab-background-color: transparent;
    --dv-activegroup-visiblepanel-tab-color: var(--foreground);
    --dv-activegroup-hiddenpanel-tab-color: var(--muted-foreground);

    /* Inactive group tabs */
    --dv-inactivegroup-visiblepanel-tab-background-color: var(--background);
    --dv-inactivegroup-hiddenpanel-tab-background-color: transparent;
    --dv-inactivegroup-visiblepanel-tab-color: var(--muted-foreground);
    --dv-inactivegroup-hiddenpanel-tab-color: var(--muted-foreground);

    /* Borders and separators */
    --dv-tab-divider-color: transparent;
    --dv-separator-border: var(--border);
    --dv-paneview-header-border-color: var(--border);

    /* Drag and drop */
    --dv-drag-over-background-color: var(--accent);
    --dv-drag-over-border-color: var(--primary);

    /* Sash (resize handles) */
    --dv-sash-color: transparent;
    --dv-active-sash-color: var(--primary);

    /* Misc */
    --dv-icon-hover-background-color: var(--accent);
    --dv-tabs-and-actions-container-height: 36px;
    --dv-tabs-and-actions-container-font-size: 13px;
  }

  /* Tab bar container - minimal bottom border */
  :global(.axonotes-dockview-theme .dv-tabs-and-actions-container) {
    border-bottom: 1px solid var(--border);
  }

  /* Tab styling - clean and minimal */
  :global(.axonotes-dockview-theme .dv-tab) {
    font-size: 0.8125rem;
    font-weight: 500;
    padding: 0 0.75rem;
    border-radius: 0;
    transition:
      color 150ms ease,
      background-color 150ms ease;
  }

  :global(.axonotes-dockview-theme .dv-tab:hover) {
    background-color: var(--accent);
  }

  /* Active tab indicator - bottom border instead of background */
  :global(.axonotes-dockview-theme .dv-activegroup .dv-tab.dv-active-tab) {
    box-shadow: inset 0 -2px 0 var(--primary);
  }

  /* Panel content area */
  :global(.axonotes-dockview-theme .dv-content) {
    background-color: var(--background);
  }

  /* Close button - subtle */
  :global(.axonotes-dockview-theme .dv-default-tab-content-close) {
    opacity: 0;
    transition: opacity 150ms ease;
  }

  :global(
    .axonotes-dockview-theme .dv-tab:hover .dv-default-tab-content-close
  ) {
    opacity: 0.6;
  }

  :global(.axonotes-dockview-theme .dv-default-tab-content-close:hover) {
    opacity: 1;
  }

  /* Svelte panel wrapper */
  :global(.dockview-svelte-panel) {
    background-color: var(--background);
  }

  /* Remove extra borders on groups */
  :global(.axonotes-dockview-theme .dv-groupview) {
    border: none;
  }

  /* Watermark area (when no panels) */
  :global(.axonotes-dockview-theme .dv-watermark) {
    background-color: var(--background);
  }
</style>
