import tseslint from "typescript-eslint";
import sveltePlugin from "eslint-plugin-svelte";
import svelteParser from "svelte-eslint-parser";
import prettier from "eslint-config-prettier";
import globals from "globals";

export default tseslint.config(
    // 1. IGNORE PATTERNS: Explicitly ignore build artifacts and generated files.
    {
        ignores: [
            "**/build/**",
            "**/dist/**",
            "**/.svelte-kit/**",
            "**/SpacetimeDB/**",
            "**/bin/**",
        ],
    },

    // 2. GLOBAL CONFIG: Apply to all relevant files.
    {
        files: ["**/*.js", "**/*.ts", "**/*.svelte"],
        languageOptions: {
            globals: {
                ...globals.browser,
                ...globals.node,
            },
        },
    },

    // 3. TYPESCRIPT CONFIG: Apply TS parser and rules to TS files.
    {
        files: ["**/*.ts"],
        extends: [...tseslint.configs.recommended],
        rules: {
            "@typescript-eslint/no-unused-vars": [
                "error",
                {
                    argsIgnorePattern: "^_",
                    varsIgnorePattern: "^_",
                    caughtErrorsIgnorePattern: "^_",
                },
            ],
        },
    },

    // 4. SVELTE CONFIG: Apply Svelte parser, which then uses the TS parser
    // for the <script> blocks.
    {
        files: ["**/*.svelte"],
        extends: [
            ...sveltePlugin.configs.recommended,
            ...tseslint.configs.recommended,
        ],
        languageOptions: {
            parser: svelteParser,
            parserOptions: {
                parser: tseslint.parser,
            },
        },
        rules: {
            "@typescript-eslint/no-unused-vars": [
                "error",
                {
                    argsIgnorePattern: "^_",
                    varsIgnorePattern: "^_",
                    caughtErrorsIgnorePattern: "^_",
                },
            ],
        },
    },

    // 5. Must be the VERY LAST entry to disable conflicting style rules.
    prettier
);
