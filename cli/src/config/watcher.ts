import {watch, FSWatcher} from "chokidar";
import {resolve} from "node:path";
import {ConfigLoader} from "./loader.js";
import {FileGenerator} from "./generator.js";
import {ensureEnvironmentSetup} from "./functions.js";
import type {WatchOptions, ConfigDefinition} from "./types.js";

export class ConfigWatcher {
    private watcher?: FSWatcher;
    private loader: ConfigLoader;
    private generator: FileGenerator;
    private debounceTimer?: NodeJS.Timeout;
    private isGenerating = false;

    constructor(private options: WatchOptions = {}) {
        this.loader = new ConfigLoader(options);
        this.generator = new FileGenerator();
    }

    async start(): Promise<void> {
        try {
            console.log("🔍 Starting configuration watcher...");

            ensureEnvironmentSetup();

            const config = await this.loader.loadConfig();
            const validation = this.loader.validateConfig(config);

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

            await this.generateInitial(config);

            this.setupWatcher(config);

            console.log("👀 Watching for changes... (Press Ctrl+C to stop)");
            console.log(`📁 Root directory: ${this.loader.getRootDir()}`);
            console.log(`⚙️  Config file: ${this.loader.getConfigPath()}`);
        } catch (error) {
            console.error(
                "❌ Failed to start watcher:",
                error instanceof Error ? error.message : error
            );
            process.exit(1);
        }
    }

    async stop(): Promise<void> {
        if (this.debounceTimer) {
            clearTimeout(this.debounceTimer);
        }

        if (this.watcher) {
            await this.watcher.close();
            console.log("👋 Configuration watcher stopped");
        }
    }

    private async generateInitial(config: ConfigDefinition): Promise<void> {
        console.log("⚡ Generating initial configuration files...");

        const result = await this.generator.generateAll(config);

        this.logGenerationResult(result);

        if (!result.success) {
            console.error("❌ Initial generation failed");
            process.exit(1);
        }
    }

    private setupWatcher(config: ConfigDefinition): void {
        const watchPaths = this.getWatchPaths(config);

        this.watcher = watch(watchPaths, {
            ignoreInitial: true,
            persistent: true,
            followSymlinks: false,
            atomic: true,
            usePolling: false,
            interval: 100,
            binaryInterval: 300,
        });

        this.watcher.on("change", (path) => this.handleFileChange(path));
        this.watcher.on("add", (path) => this.handleFileChange(path));
        this.watcher.on("error", (error) =>
            this.handleWatchError(error as Error)
        );

        console.log("📂 Watching files:");
        watchPaths.forEach((path) => console.log(`   - ${path}`));
    }

    private getWatchPaths(config: ConfigDefinition): string[] {
        const paths = new Set<string>();

        paths.add(this.loader.getConfigPath());

        paths.add(resolve(".env"));

        if (config.watch) {
            config.watch.forEach((watchPath) => {
                const resolvedPath = resolve(watchPath);
                paths.add(resolvedPath);
            });
        }

        return Array.from(paths);
    }

    private handleFileChange(path: string): void {
        if (this.isGenerating) {
            return;
        }

        const debounceMs = this.options.debounceMs || 300;

        if (this.debounceTimer) {
            clearTimeout(this.debounceTimer);
        }

        this.debounceTimer = setTimeout(async () => {
            await this.regenerateFiles(path);
        }, debounceMs);
    }

    private async regenerateFiles(changedPath: string): Promise<void> {
        if (this.isGenerating) {
            return;
        }

        this.isGenerating = true;

        try {
            console.log(`\n🔄 File changed: ${changedPath}`);
            console.log("⚡ Regenerating configuration files...");

            ensureEnvironmentSetup();

            const config = await this.loader.loadConfig();
            const validation = this.loader.validateConfig(config);

            if (!validation.valid) {
                console.error("❌ Configuration validation failed:");
                validation.errors.forEach((error) =>
                    console.error(`   - ${error}`)
                );
                return;
            }

            if (validation.warnings.length > 0) {
                console.warn("⚠️  Configuration warnings:");
                validation.warnings.forEach((warning) =>
                    console.warn(`   - ${warning}`)
                );
            }

            const result = await this.generator.generateAll(config);
            this.logGenerationResult(result);

            console.log("✅ Regeneration complete\n");
        } catch (error) {
            console.error(
                "❌ Failed to regenerate files:",
                error instanceof Error ? error.message : error
            );
        } finally {
            this.isGenerating = false;
        }
    }

    private handleWatchError(error: Error): void {
        console.error("❌ Watcher error:", error.message);
    }

    private logGenerationResult(result: any): void {
        if (result.success) {
            console.log(
                `✅ Generated ${result.results.length} files successfully`
            );

            result.results.forEach((fileResult: any) => {
                if (fileResult.success) {
                    const status = fileResult.created ? "created" : "updated";
                    console.log(`   📄 ${status}: ${fileResult.path}`);
                }
            });
        } else {
            console.error(
                `❌ Generation failed with ${result.errors.length} errors:`
            );
            result.errors.forEach((error: string) =>
                console.error(`   - ${error}`)
            );

            result.results.forEach((fileResult: any) => {
                if (!fileResult.success) {
                    console.error(
                        `   ❌ ${fileResult.path}: ${fileResult.error}`
                    );
                }
            });
        }
    }
}

process.on("SIGINT", async () => {
    console.log("\n🛑 Received interrupt signal...");
    process.exit(0);
});

process.on("SIGTERM", async () => {
    console.log("\n🛑 Received termination signal...");
    process.exit(0);
});
