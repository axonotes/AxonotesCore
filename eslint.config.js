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
        languageOptions: {
            parserOptions: {
                project: true,
                tsconfigRootDir: import.meta.dirname,
                extraFileExtensions: [".svelte"],
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

    // 4. SVELTE-SPECIFIC CONFIG
    // This block layers Svelte-specific parsing and rules on top.
    // It inherits the type-aware settings from the block above.
    {
        files: ["**/*.svelte"],
        extends: [...sveltePlugin.configs.recommended],
        languageOptions: {
            parser: svelteParser,
            parserOptions: {
                parser: tseslint.parser,
            },
        },
    },

    // 5. PRETTIER: Must be the VERY LAST entry.
    prettier
);
