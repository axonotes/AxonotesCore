import {pathToFileURL} from "node:url";
import {resolve, dirname} from "node:path";
import {existsSync, readFileSync} from "node:fs";
import type {
    ConfigDefinition,
    ValidationResult,
    ConfigOptions,
} from "./types.js";

export class ConfigLoader {
    private configPath: string;
    private rootDir: string;

    constructor(options: ConfigOptions = {}) {
        this.rootDir = this.findMonorepoRoot(options.rootDir);
        this.ensureCorrectWorkingDirectory();
        this.configPath = options.configPath || this.findConfigFile();
    }

    private findMonorepoRoot(startDir?: string): string {
        const currentDir = startDir || process.cwd();

        const indicators = [
            "axonotes.config.ts",
            "axonotes.config.js",
            "package.json",
        ];

        let dir = resolve(currentDir);

        while (dir !== dirname(dir)) {
            for (const indicator of indicators) {
                if (existsSync(resolve(dir, indicator))) {
                    const packageJsonPath = resolve(dir, "package.json");
                    if (existsSync(packageJsonPath)) {
                        return dir;
                    }
                }
            }
            dir = dirname(dir);
        }

        if (currentDir.includes("/cli")) {
            const parentDir = resolve(currentDir, "..");
            if (existsSync(resolve(parentDir, "package.json"))) {
                return parentDir;
            }
        }

        throw new Error(
            "Could not find monorepo root. Please run from the project root directory."
        );
    }

    private ensureCorrectWorkingDirectory(): void {
        if (process.cwd() !== this.rootDir) {
            console.log(`📁 Changing working directory to: ${this.rootDir}`);
            process.chdir(this.rootDir);
        }
    }

    private findConfigFile(): string {
        const candidates = [
            resolve(this.rootDir, "../axonotes.config.ts"),
            resolve(this.rootDir, "../axonotes.config.js"),
            resolve(this.rootDir, "axonotes.config.ts"),
            resolve(this.rootDir, "axonotes.config.js"),
        ];

        for (const candidate of candidates) {
            if (existsSync(candidate)) {
                return candidate;
            }
        }

        throw new Error(
            `No configuration file found. Tried: ${candidates.join(", ")}`
        );
    }

    async loadConfig(): Promise<ConfigDefinition> {
        try {
            const configUrl = pathToFileURL(this.configPath).href;

            const module = await import(configUrl);

            const config = module.default || module;

            if (typeof config === "function") {
                return config();
            }

            if (typeof config === "object" && config !== null) {
                return config as ConfigDefinition;
            }

            throw new Error(
                "Configuration file must export a config object or function"
            );
        } catch (error) {
            if (error instanceof Error) {
                throw new Error(
                    `Failed to load config from ${this.configPath}: ${error.message}`
                );
            }
            throw error;
        }
    }

    getConfigContent(): string {
        try {
            return readFileSync(this.configPath, "utf-8");
        } catch (error) {
            throw new Error(
                `Failed to read config file: ${error instanceof Error ? error.message : error}`
            );
        }
    }

    validateConfig(config: ConfigDefinition): ValidationResult {
        const errors: string[] = [];
        const warnings: string[] = [];

        if (!config.targets || Object.keys(config.targets).length === 0) {
            errors.push("Configuration must define at least one target");
        }

        for (const [name, target] of Object.entries(config.targets || {})) {
            if (!target.type) {
                errors.push(`Target '${name}' must specify a type`);
            }

            if (!target.path) {
                errors.push(`Target '${name}' must specify a path`);
            }

            if (!target.variables) {
                warnings.push(`Target '${name}' has no variables defined`);
            }

            if (target.type === "template" && !target.template) {
                errors.push(
                    `Target '${name}' with type 'template' must specify a template file`
                );
            }

            if (target.type === "template" && target.template) {
                const templatePath = resolve(
                    dirname(this.configPath),
                    target.template
                );
                if (!existsSync(templatePath)) {
                    errors.push(`Template file not found: ${templatePath}`);
                }
            }

            const validTypes = ["env", "json", "toml", "yaml", "template"];
            if (!validTypes.includes(target.type)) {
                errors.push(
                    `Target '${name}' has invalid type '${target.type}'. Valid types: ${validTypes.join(", ")}`
                );
            }
        }

        if (config.watch) {
            if (!Array.isArray(config.watch)) {
                errors.push("watch option must be an array of file paths");
            } else {
                for (const watchPath of config.watch) {
                    if (typeof watchPath !== "string") {
                        errors.push("watch paths must be strings");
                    }
                }
            }
        }

        return {
            valid: errors.length === 0,
            errors,
            warnings,
        };
    }

    getConfigPath(): string {
        return this.configPath;
    }

    getRootDir(): string {
        return this.rootDir;
    }
}
