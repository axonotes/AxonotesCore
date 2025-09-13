/**
 * Parameters for the copy action.
 */
interface CopyParams {
    /** The text to be copied to the clipboard. */
    text: string;
    /** The text to display on the element after a successful copy. */
    successText?: string;
    /** The text to display on the element by default. */
    defaultText?: string;
    /** The duration in milliseconds to show the success text. */
    duration?: number;
}

/**
 * Svelte action that copies `params.text` to the clipboard on click and briefly replaces the element's text content with a success message.
 *
 * The action optionally initializes the element's text content from `params.defaultText`. On a successful copy it sets the element text to `params.successText` (defaults to `"Copied!"`) for `params.duration` milliseconds (defaults to `2000`), then restores the original/default text. If the copy fails the error is logged to the console.
 *
 * @param node - The HTMLElement to attach the action to.
 * @param params - Configuration for the copy behavior. Important fields:
 *   - `text` (string): text to copy (required).
 *   - `successText` (string): message shown after a successful copy (default: `"Copied!"`).
 *   - `defaultText` (string): initial text to set on the element (optional).
 *   - `duration` (number): milliseconds to show the success text before reverting (default: `2000`).
 * @returns An object with lifecycle methods:
 *   - `update(newParams: CopyParams)` — replace the action parameters (updates element text if `defaultText` is provided).
 *   - `destroy()` — removes the click listener and clears any pending revert timer.
 */
export function copy(node: HTMLElement, params: CopyParams) {
    let timer: number;

    // Set initial text content if provided
    if (params.defaultText) {
        node.textContent = params.defaultText;
    }

    const handleClick = async () => {
        if (!params.text) return;

        try {
            await navigator.clipboard.writeText(params.text);

            // Provide visual feedback
            const originalText = params.defaultText || node.textContent;
            const successText = params.successText || "Copied!";
            const duration = params.duration || 2000;

            node.textContent = successText;

            // Revert to original text after a delay
            clearTimeout(timer);
            timer = window.setTimeout(() => {
                node.textContent = originalText;
            }, duration);
        } catch (err) {
            console.error("Failed to copy text: ", err);
        }
    };

    node.addEventListener("click", handleClick);

    return {
        // This function is called when the parameters change
        update(newParams: CopyParams) {
            params = newParams;
            // If the default text is part of the new params, update the node
            if (newParams.defaultText) {
                node.textContent = newParams.defaultText;
            }
        },
        // This function is called when the element is removed from the DOM
        destroy() {
            node.removeEventListener("click", handleClick);
            clearTimeout(timer);
        },
    };
}
