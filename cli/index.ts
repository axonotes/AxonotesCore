#!/usr/bin/env bun

import {Command} from "commander";
import {ConfigLoader} from "./src/config/loader.js";
import {FileGenerator} from "./src/config/generator.js";
import {ConfigWatcher} from "./src/config/watcher.js";
import {ensureEnvironmentSetup} from "./src/config/functions.js";
import type {GenerateOptions, WatchOptions} from "./src/config/types.js";

const program = new Command();

program
    .name("axonotes-config")
    .description(
        "TypeScript configuration management system for axonotes monorepo"
    )
    .version("0.1.0");

program
    .command("generate")
    .description("Generate configuration files from axonotes.config.ts")
    .option("-t, --target <name>", "Generate only the specified target")
    .option(
        "-d, --dry-run",
        "Show what would be generated without creating files"
    )
    .option("-v, --verbose", "Verbose output")
    .action(async (options: GenerateOptions) => {
        try {
            console.log("⚡ Generating configuration files...");

            const loader = new ConfigLoader(options);
            const configContent = loader.getConfigContent();

            ensureEnvironmentSetup(configContent);

            const generator = new FileGenerator();

            const config = await loader.loadConfig();
            const validation = loader.validateConfig(config);

            if (!validation.valid) {
                console.error("❌ Configuration validation failed:");
                validation.errors.forEach((error) =>
                    console.error(`   - ${error}`)
                );
                process.exit(1);
            }

            if (validation.warnings.length > 0) {
                console.warn("⚠️  Configuration warnings:");
                validation.warnings.forEach((warning) =>
                    console.warn(`   - ${warning}`)
                );
            }

            if (options.target) {
                if (!config.targets[options.target]) {
                    console.error(
                        `❌ Target '${options.target}' not found in configuration`
                    );
                    process.exit(1);
                }

                if (options.dryRun) {
                    console.log(
                        `📋 Would generate target '${options.target}' -> ${config.targets[options.target].path}`
                    );
                    return;
                }

                const result = await generator.generateTarget(
                    options.target,
                    config.targets[options.target],
                    config.globals || {}
                );

                if (result.success) {
                    const status = result.created ? "created" : "updated";
                    console.log(`✅ Target ${status}: ${result.path}`);
                } else {
                    console.error(
                        `❌ Failed to generate target '${options.target}': ${result.error}`
                    );
                    process.exit(1);
                }
            } else {
                if (options.dryRun) {
                    console.log("📋 Would generate the following files:");
                    Object.entries(config.targets).forEach(([name, target]) => {
                        console.log(`   - ${name} -> ${target.path}`);
                    });
                    return;
                }

                const result = await generator.generateAll(config);

                if (result.success) {
                    console.log(
                        `✅ Generated ${result.results.length} files successfully`
                    );
                    result.results.forEach((fileResult) => {
                        if (fileResult.success) {
                            const status = fileResult.created
                                ? "created"
                                : "updated";
                            console.log(`   📄 ${status}: ${fileResult.path}`);
                        }
                    });
                } else {
                    console.error(
                        `❌ Generation failed with ${result.errors.length} errors:`
                    );
                    result.errors.forEach((error) =>
                        console.error(`   - ${error}`)
                    );
                    process.exit(1);
                }
            }
        } catch (error) {
            console.error(
                "❌ Generation failed:",
                error instanceof Error ? error.message : error
            );
            process.exit(1);
        }
    });

program
    .command("watch")
    .description("Watch for changes and auto-regenerate configuration files")
    .option("--debounce <ms>", "Debounce delay in milliseconds", "300")
    .option("-v, --verbose", "Verbose output")
    .action(async (options: WatchOptions & {debounce?: string}) => {
        try {
            const watchOptions: WatchOptions = {
                ...options,
                debounceMs: options.debounce
                    ? parseInt(options.debounce, 10)
                    : 300,
            };

            const watcher = new ConfigWatcher(watchOptions);
            await watcher.start();
        } catch (error) {
            console.error(
                "❌ Watch failed:",
                error instanceof Error ? error.message : error
            );
            process.exit(1);
        }
    });

program
    .command("validate")
    .description("Validate the configuration file")
    .option("-v, --verbose", "Verbose output")
    .action(async (options) => {
        try {
            console.log("🔍 Validating configuration...");

            const loader = new ConfigLoader(options);
            const configContent = loader.getConfigContent();

            ensureEnvironmentSetup(configContent);

            const config = await loader.loadConfig();
            const validation = loader.validateConfig(config);

            if (validation.valid) {
                console.log("✅ Configuration is valid");

                if (validation.warnings.length > 0) {
                    console.warn(
                        `⚠️  Found ${validation.warnings.length} warnings:`
                    );
                    validation.warnings.forEach((warning) =>
                        console.warn(`   - ${warning}`)
                    );
                }

                console.log(
                    `📊 Found ${Object.keys(config.targets).length} targets:`
                );
                Object.entries(config.targets).forEach(([name, target]) => {
                    console.log(
                        `   - ${name} (${target.type}) -> ${target.path}`
                    );
                });
            } else {
                console.error(
                    `❌ Configuration is invalid with ${validation.errors.length} errors:`
                );
                validation.errors.forEach((error) =>
                    console.error(`   - ${error}`)
                );

                if (validation.warnings.length > 0) {
                    console.warn(
                        `⚠️  Also found ${validation.warnings.length} warnings:`
                    );
                    validation.warnings.forEach((warning) =>
                        console.warn(`   - ${warning}`)
                    );
                }

                process.exit(1);
            }
        } catch (error) {
            console.error(
                "❌ Validation failed:",
                error instanceof Error ? error.message : error
            );
            process.exit(1);
        }
    });

program
    .command("init")
    .description("Create a template axonotes.config.ts file")
    .option("-f, --force", "Overwrite existing configuration file")
    .action(async (options) => {
        try {
            console.log("🚀 Initializing axonotes configuration...");

            const configPath = "axonotes.config.ts";

            if (!options.force && require("fs").existsSync(configPath)) {
                console.error(
                    `❌ Configuration file already exists at ${configPath}`
                );
                console.error("   Use --force to overwrite");
                process.exit(1);
            }

            const templateConfig = `import { defineConfig, env, generateJwtKeys } from './cli/src/config/functions.js';

// Generate JWT keys if they don't exist
const keys = generateJwtKeys();

// Flexible endpoint handling
const dashboardOrigin = env.DASHBOARD_ORIGIN || 'http://localhost:5173';
const spacetimeEndpoint = env.SPACETIME_ENDPOINT || 'ws://localhost:3000';
const dashboardPort = env.DASHBOARD_PORT || '5173';
const spacetimePort = env.SPACETIME_PORT || '3000';

export default defineConfig({
  watch: ['.env', 'axonotes.config.ts'],
  
  globals: {
    dashboardOrigin,
    spacetimeEndpoint,
    dashboardPort,
    spacetimePort,
    moduleName: 'axonotes',
    keys,
  },
  
  targets: {
    dashboard: {
      type: 'env',
      path: 'dashboard/.env',
      variables: {
        WORKOS_CLIENT_ID: env.WORKOS_CLIENT_ID || '',
        WORKOS_API_KEY: env.WORKOS_API_KEY || '',
        WORKOS_REDIRECT_URI: \`\${dashboardOrigin}/auth/callback\`,
        WORKOS_SESSION_ID_COOKIE_NAME: 'workos_session_id',
        JWT_PRIVATE_KEY_BASE64: keys.privateKey,
        JWT_PUBLIC_KEY_BASE64: keys.publicKey,
        JWT_KEY_ID: keys.keyId,
        JWT_ISSUER: dashboardOrigin,
        REFRESH_TOKEN_COOKIE_NAME: 'axonotes_auth_refresh_token',
        PUBLIC_ACCESS_TOKEN_COOKIE_NAME: 'axonotes_auth_access_token',
        WEB_REFRESH_TOKEN_EXPIRES_IN: '7d',
        WEB_ACCESS_TOKEN_EXPIRES_IN: '15m',
        PUBLIC_SPACETIME_WS: spacetimeEndpoint,
        PUBLIC_SPACETIME_MODULE_NAME: 'axonotes',
        DESKTOP_AUTH_SESSION_TTL: '300',
        DESKTOP_AUTH_CALLBACK_URL: \`\${dashboardOrigin}/auth/desktop/callback\`,
        DESKTOP_REFRESH_TOKEN_EXPIRES_IN: '30d',
        DESKTOP_ACCESS_TOKEN_EXPIRES_IN: '1h',
        DEVICE_CODE_TTL_MS: '600000',
        DEVICE_POLL_INTERVAL: '5',
        RATE_LIMIT_CACHE_TTL: '3600',
        RATE_LIMIT_CHECK_PERIOD: '60',
        DEVICE_FLOW_WINDOW_MS: '60000',
        DEVICE_FLOW_MAX_REQUESTS: '15',
        DEVICE_FLOW_WEIGHT: '3',
        DEVICE_POLL_WINDOW_MS: '60000',
        DEVICE_POLL_MAX_REQUESTS: '15',
        DEVICE_POLL_WEIGHT: '1',
        TOKEN_EXCHANGE_WINDOW_MS: '60000',
        TOKEN_EXCHANGE_MAX_REQUESTS: '15',
        TOKEN_EXCHANGE_WEIGHT: '2',
        TOKEN_REFRESH_WINDOW_MS: '60000',
        TOKEN_REFRESH_MAX_REQUESTS: '30',
        TOKEN_REFRESH_WEIGHT: '1',
      }
    },
    
    app: {
      type: 'json',
      path: 'app/config.json',
      variables: {
        apiUrl: \`\${dashboardOrigin}/api\`,
        spacetimeWs: spacetimeEndpoint,
        authCallbackUrl: \`\${dashboardOrigin}/auth/desktop/callback\`,
        moduleName: 'axonotes',
      }
    },
    
    server: {
      type: 'toml',
      path: 'server/config.toml',
      variables: {
        database_url: env.DATABASE_URL || 'sqlite:axonotes.db',
        jwt_public_key_base64: keys.publicKey,
        jwt_key_id: keys.keyId,
        module_name: 'axonotes',
      }
    }
  }
});
`;

            require("fs").writeFileSync(configPath, templateConfig);

            console.log(`✅ Created ${configPath}`);
            console.log("📝 Next steps:");
            console.log(
                "   1. Edit the configuration file to match your needs"
            );
            console.log("   2. Configure environment variables in .env");
            console.log(
                '   3. Run "bun cli generate" to create configuration files'
            );
        } catch (error) {
            console.error(
                "❌ Initialization failed:",
                error instanceof Error ? error.message : error
            );
            process.exit(1);
        }
    });

program.parse();
