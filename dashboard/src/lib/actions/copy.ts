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
 * A Svelte action that copies a given text to the clipboard when the node is clicked.
 * It also provides visual feedback by temporarily changing the node's text content.
 *
 * @param {HTMLElement} node - The element to attach the action to.
 * @param {CopyParams} params - The configuration for the copy action.
 *
 * @example
 * ```svelte
 * <script>
 *   import { copy } from '$lib/actions/copy';
 *   let someText = "Hello, world!";
 * </script>
 *
 * <button use:copy={{ text: someText }}>
 *   Copy
 * </button>
 *
 * <button use:copy={{ text: "Customized", successText: "Done!", duration: 1000 }}>
 *   Copy Me
 * </button>
 * ```
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
