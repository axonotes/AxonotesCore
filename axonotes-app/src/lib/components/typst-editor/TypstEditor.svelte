<!--
  TypstEditor.svelte
  
  Mounts a CodeMirror 6 editor instance and bridges it to the TypstEditorContext.
  
  Responsibilities:
  - Create and configure the CM6 EditorView
  - Wire the updateListener to detect user changes (forwarded to context)
  - Wire a transactionFilter to block edits on lines locked by other users
  - Track cursor position for lock management
  - Handle blur for lock release
  - Theme the editor to match the app's light/dark mode
-->
<script lang="ts">
  import {onMount, onDestroy} from "svelte";
  import {
    EditorView,
    keymap,
    lineNumbers,
    drawSelection,
    highlightActiveLine,
    highlightSpecialChars,
    type ViewUpdate,
  } from "@codemirror/view";
  import {EditorState, type Extension} from "@codemirror/state";
  import {
    defaultKeymap,
    history,
    historyKeymap,
    indentWithTab,
  } from "@codemirror/commands";
  import {getTypstEditorContext, isRemoteChange} from "./typstEditorContext";

  interface Props {
    initialText: string;
  }

  let {initialText}: Props = $props();

  let container: HTMLDivElement;
  let view: EditorView | null = null;
  const ctx = getTypstEditorContext();

  onMount(() => {
    // --- Update listener: forward doc changes to the context ---
    const updateListener = EditorView.updateListener.of(
      (update: ViewUpdate) => {
        if (update.docChanged) {
          // Skip remote changes (dispatched by the context for sync/collab)
          const hasRemote = update.transactions.some((tr) =>
            tr.annotation(isRemoteChange)
          );
          if (!hasRemote) {
            ctx.handleDocChanged(update);
          }
        }

        // Track cursor line for lock management
        if (update.selectionSet || update.docChanged) {
          const mainSel = update.state.selection.main;
          const line = update.state.doc.lineAt(mainSel.head);
          ctx.handleCursorLine(line.number); // 1-based
        }
      }
    );

    // --- Transaction filter: reject edits to lines locked by other users ---
    const lockFilter = EditorState.transactionFilter.of((tr) => {
      // Always allow remote changes and non-doc changes
      if (tr.annotation(isRemoteChange)) return tr;
      if (!tr.docChanged) return tr;

      // Check if any changed lines are locked by another user
      let blocked = false;
      tr.changes.iterChanges((fromA: number, toA: number) => {
        if (blocked) return;

        const doc = tr.startState.doc;

        // Determine which lines in the old doc this change touches.
        // Count newlines in the replaced range to get the line span.
        const oldText = fromA < toA ? doc.sliceString(fromA, toA) : "";
        let newlines = 0;
        for (let i = 0; i < oldText.length; i++) {
          if (oldText.charCodeAt(i) === 10) newlines++;
        }
        const lineCount = 1 + newlines;
        const firstLineNum = doc.lineAt(fromA).number; // 1-based

        for (let i = 0; i < lineCount; i++) {
          const lineIdx = firstLineNum - 1 + i; // 0-based
          if (ctx.isLineLockedByOther(lineIdx)) {
            blocked = true;
            return;
          }
        }
      });

      // Return empty array to reject the transaction
      return blocked ? [] : tr;
    });

    // --- Theme: match the app's CSS variables for light/dark mode ---
    const theme = EditorView.theme({
      "&": {
        height: "100%",
        fontSize: "14px",
        backgroundColor: "transparent",
      },
      ".cm-content": {
        fontFamily:
          'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
        padding: "16px 0",
        caretColor: "var(--foreground)",
      },
      ".cm-line": {
        padding: "0 16px",
      },
      "&.cm-focused .cm-cursor": {
        borderLeftColor: "var(--foreground)",
        borderLeftWidth: "2px",
      },
      "&.cm-focused": {
        outline: "none",
      },
      ".cm-gutters": {
        backgroundColor: "transparent",
        borderRight:
          "1px solid color-mix(in oklch, var(--border) 50%, transparent)",
        color: "var(--muted-foreground)",
        minWidth: "3em",
      },
      ".cm-activeLineGutter": {
        backgroundColor: "transparent",
        color: "var(--foreground)",
      },
      ".cm-activeLine": {
        backgroundColor: "color-mix(in oklch, var(--accent) 30%, transparent)",
      },
      ".cm-selectionBackground": {
        backgroundColor:
          "color-mix(in oklch, var(--primary) 25%, transparent) !important",
      },
      "&.cm-focused .cm-selectionBackground": {
        backgroundColor:
          "color-mix(in oklch, var(--primary) 30%, transparent) !important",
      },
      ".cm-scroller": {
        overflow: "auto",
      },
    });

    // --- Assemble extensions ---
    const extensions: Extension[] = [
      lineNumbers(),
      highlightActiveLine(),
      highlightSpecialChars(),
      drawSelection(),
      history(),
      EditorView.lineWrapping,
      keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
      updateListener,
      lockFilter,
      theme,
    ];

    // --- Create the editor ---
    const state = EditorState.create({
      doc: initialText,
      extensions,
    });

    view = new EditorView({
      state,
      parent: container,
    });

    // Register the view with the context so it can dispatch remote changes
    ctx.setView(view);

    // Handle blur: release any locks
    view.contentDOM.addEventListener("blur", handleBlur);
  });

  function handleBlur(): void {
    ctx.handleBlur();
  }

  onDestroy(() => {
    if (view) {
      view.contentDOM.removeEventListener("blur", handleBlur);
      view.destroy();
      view = null;
    }
  });
</script>

<div
  bind:this={container}
  class="typst-editor h-full w-full overflow-hidden"
></div>

<style>
  .typst-editor {
    /* Ensure the CM6 editor fills the container */
    display: flex;
    flex-direction: column;
  }
  .typst-editor :global(.cm-editor) {
    flex: 1;
    height: 100%;
  }
</style>
