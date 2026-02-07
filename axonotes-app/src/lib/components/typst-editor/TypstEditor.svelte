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
    Decoration,
    WidgetType,
    keymap,
    lineNumbers,
    drawSelection,
    highlightActiveLine,
    highlightSpecialChars,
    type ViewUpdate,
    type DecorationSet,
  } from "@codemirror/view";
  import {
    EditorState,
    StateField,
    RangeSet,
    type Extension,
    type Range,
  } from "@codemirror/state";
  import {
    defaultKeymap,
    history,
    historyKeymap,
    indentWithTab,
  } from "@codemirror/commands";
  import {
    getTypstEditorContext,
    isRemoteChange,
    setLineLocks,
    flashLines,
    type LineLockState,
  } from "./typstEditorContext";

  interface Props {
    initialText: string;
  }

  let {initialText}: Props = $props();

  let container: HTMLDivElement;
  let view: EditorView | null = null;
  const ctx = getTypstEditorContext();

  onMount(() => {
    // --- Name tag widget for locked lines ---
    class LockNameTagWidget extends WidgetType {
      username: string;
      color: string;

      constructor(username: string, color: string) {
        super();
        this.username = username;
        this.color = color;
      }

      toDOM(): HTMLElement {
        const tag = document.createElement("span");
        tag.className = "cm-lock-nametag";
        tag.textContent = this.username;
        tag.style.backgroundColor = this.color;
        return tag;
      }

      eq(other: LockNameTagWidget): boolean {
        return this.username === other.username && this.color === other.color;
      }

      ignoreEvent(): boolean {
        return true;
      }
    }

    // --- Lock decoration state field ---
    const lockDecoField = StateField.define<{
      locks: LineLockState[];
      decos: DecorationSet;
    }>({
      create() {
        return {locks: [], decos: Decoration.none};
      },

      update(value, tr) {
        let locks = value.locks;

        for (const e of tr.effects) {
          if (e.is(setLineLocks)) {
            locks = e.value;
          }
        }

        // Rebuild decorations if locks changed or doc changed
        if (locks !== value.locks || tr.docChanged) {
          const ranges: Range<Decoration>[] = [];
          const doc = tr.state.doc;

          for (const lock of locks) {
            const lineNum = lock.lineIndex + 1; // 1-based
            if (lineNum < 1 || lineNum > doc.lines) continue;
            const line = doc.line(lineNum);

            // Line decoration (background + left border)
            const lineClass =
              lock.kind === "other-locked"
                ? "cm-line-other-locked"
                : lock.kind === "other-focused"
                  ? "cm-line-other-focused"
                  : lock.kind === "own-locked"
                    ? "cm-line-own-locked"
                    : "cm-line-own-focused";

            ranges.push(
              Decoration.line({
                attributes: {
                  class: lineClass,
                  style: `--lock-color: ${lock.color};`,
                },
              }).range(line.from)
            );

            // Name tag widget for other users' locked/focused lines
            if (
              (lock.kind === "other-locked" || lock.kind === "other-focused") &&
              lock.username
            ) {
              ranges.push(
                Decoration.widget({
                  widget: new LockNameTagWidget(lock.username, lock.color),
                  side: -1, // Before the line content
                }).range(line.from)
              );
            }
          }

          // Sort by position (required by CM6)
          ranges.sort(
            (a, b) => a.from - b.from || a.value.startSide - b.value.startSide
          );

          return {
            locks,
            decos: RangeSet.of(ranges),
          };
        }

        return value;
      },

      provide: (f) => EditorView.decorations.from(f, (val) => val.decos),
    });

    // --- Flash decoration state field ---
    const flashDecoField = StateField.define<{
      lines: number[];
      decos: DecorationSet;
      timer: ReturnType<typeof setTimeout> | null;
    }>({
      create() {
        return {lines: [], decos: Decoration.none, timer: null};
      },

      update(value, tr) {
        for (const e of tr.effects) {
          if (e.is(flashLines)) {
            const flashLineIndices = e.value as number[];

            // Empty array = clear flash
            if (flashLineIndices.length === 0) {
              if (value.timer) clearTimeout(value.timer);
              return {lines: [], decos: Decoration.none, timer: null};
            }

            // Start flash
            const doc = tr.state.doc;
            const ranges: Range<Decoration>[] = [];

            for (const lineIdx of flashLineIndices) {
              const lineNum = lineIdx + 1;
              if (lineNum < 1 || lineNum > doc.lines) continue;
              const line = doc.line(lineNum);
              ranges.push(
                Decoration.line({
                  attributes: {class: "cm-line-flash"},
                }).range(line.from)
              );
            }

            ranges.sort((a, b) => a.from - b.from);

            // Schedule removal of the flash
            if (value.timer) clearTimeout(value.timer);

            // We can't dispatch from within update, so use setTimeout
            const currentView = view;
            const timer = setTimeout(() => {
              if (currentView) {
                currentView.dispatch({
                  effects: flashLines.of([]),
                });
              }
            }, 400);

            return {
              lines: flashLineIndices,
              decos: RangeSet.of(ranges),
              timer,
            };
          }
        }

        // If doc changed, try to map existing decorations
        if (tr.docChanged && value.lines.length > 0) {
          return {
            ...value,
            decos: value.decos.map(tr.changes),
          };
        }

        return value;
      },

      provide: (f) => EditorView.decorations.from(f, (val) => val.decos),
    });

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
      const blockedLineIndices: number[] = [];

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
            blockedLineIndices.push(lineIdx);
          }
        }
      });

      // Flash the blocked lines to give visual feedback
      if (blocked && blockedLineIndices.length > 0) {
        ctx.flashLockedLines(blockedLineIndices);
      }

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
        position: "relative",
        overflow: "visible",
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
      // --- Lock decoration styles ---
      ".cm-line-own-focused": {
        borderLeft: "2px dotted var(--lock-color, var(--primary))",
        paddingLeft: "14px !important", // Adjust for border
      },
      ".cm-line-own-locked": {
        borderLeft: "3px solid var(--lock-color, var(--primary))",
        paddingLeft: "13px !important",
      },
      ".cm-line-other-focused": {
        borderLeft: "2px dotted var(--lock-color, #6B7280)",
        paddingLeft: "14px !important",
        position: "relative",
        overflow: "visible",
      },
      ".cm-line-other-locked": {
        borderLeft: "3px solid var(--lock-color, #6B7280)",
        backgroundColor:
          "color-mix(in srgb, var(--lock-color, #6B7280) 8%, transparent)",
        paddingLeft: "13px !important",
        position: "relative",
        overflow: "visible",
      },
      // Name tag floating above the locked line
      ".cm-lock-nametag": {
        position: "absolute",
        top: "-14px",
        left: "4px",
        fontSize: "10px",
        lineHeight: "14px",
        padding: "0 6px",
        borderRadius: "4px 4px 4px 0",
        color: "white",
        whiteSpace: "nowrap",
        pointerEvents: "none",
        zIndex: "10",
        fontFamily: "system-ui, sans-serif",
      },
      // Flash animation for rejected edits
      ".cm-line-flash": {
        animation: "cm-lock-flash 400ms ease-out",
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
      lockDecoField,
      flashDecoField,
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

  /*
   * Ensure .cm-content doesn't clip absolutely-positioned name tags
   * that overflow above their line.  CM6 sets overflow: auto on .cm-scroller
   * but .cm-content itself should not clip inline widgets.
   */
  .typst-editor :global(.cm-content) {
    overflow: visible;
  }

  /*
   * Flash animation for rejected edits.
   * Uses :global() because the class is applied by CM6 decorations,
   * not by Svelte template bindings.
   */
  @keyframes -global-cm-lock-flash {
    0% {
      background-color: oklch(0.65 0.2 25 / 0.35);
    }
    100% {
      background-color: transparent;
    }
  }
</style>
