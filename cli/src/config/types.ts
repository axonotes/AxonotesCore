export type FileType = "env" | "json" | "toml" | "yaml" | "template";

export interface JWTKeys {
    privateKey: string;
    publicKey: string;
    keyId: string;
}

export interface EnvVariable {
    [key: string]: string | number | boolean;
}

export interface TargetConfig {
    type: FileType;
    path: string;
    template?: string;
    variables: Record<string, any>;
}

export interface ConfigDefinition {
    watch?: string[];
    globals?: Record<string, any>;
    targets: Record<string, TargetConfig>;
}

export interface ConfigOptions {
    configPath?: string;
    rootDir?: string;
    verbose?: boolean;
}

export interface GenerateOptions extends ConfigOptions {
    target?: string;
    dryRun?: boolean;
}

export interface WatchOptions extends ConfigOptions {
    debounceMs?: number;
}

export interface ValidationResult {
    valid: boolean;
    errors: string[];
    warnings: string[];
}

export interface FileGenerationResult {
    path: string;
    success: boolean;
    error?: string;
    created?: boolean;
    updated?: boolean;
}

export interface GenerationResult {
    success: boolean;
    results: FileGenerationResult[];
    errors: string[];
}

export interface EnvironmentState {
    exists: boolean;
    missingVariables: string[];
    hasGeneratedKeys: boolean;
    path: string;
}

export type ConfigFunction = () => ConfigDefinition;

export interface BuiltinFunctions {
    env: Record<string, string | undefined>;
    generateJwtKeys: () => JWTKeys;
    defineConfig: (config: ConfigDefinition) => ConfigDefinition;
}

export interface TemplateContext {
    variables: Record<string, any>;
    globals: Record<string, any>;
    timestamp: string;
    generatedNotice: string;
}

export interface FileHeader {
    comment: string;
    timestamp: string;
    source: string;
}
