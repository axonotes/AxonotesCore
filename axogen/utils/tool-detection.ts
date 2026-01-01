import {exec} from "@axonotes/axogen";

export interface ToolInfo {
  name: string;
  command: string;
  installed: boolean;
  version?: string;
  installUrl?: string;
}

const toolCache: Map<string, ToolInfo> = new Map();

export async function detectTool(
  name: string,
  command: string,
  versionFlag = "--version",
  installUrl?: string
): Promise<ToolInfo> {
  const cacheKey = `${name}|${command}|${versionFlag}`;
  if (toolCache.has(cacheKey)) {
    return toolCache.get(cacheKey)!;
  }

  const toolInfo = await _detectTool(name, command, versionFlag, installUrl);
  toolCache.set(cacheKey, toolInfo);
  return toolInfo;
}

async function _detectTool(
  name: string,
  command: string,
  versionFlag = "--version",
  installUrl?: string
): Promise<ToolInfo> {
  try {
    const result = await exec(`${command} ${versionFlag}`);
    return {
      name,
      command,
      installed: true,
      version: result.stdout.trim().split("\n")[0],
      installUrl,
    };
  } catch {
    return {
      name,
      command,
      installed: false,
      installUrl,
    };
  }
}

export async function throwIfToolMissing(
  name: string,
  command: string,
  versionFlag = "--version",
  installUrl?: string
): Promise<void> {
  const tool = await detectTool(name, command, versionFlag, installUrl);
  if (!tool.installed) {
    let message = `${name} is not installed.`;
    if (installUrl) {
      message += ` Please install it from ${installUrl} and try again.`;
    }
    throw new Error(message);
  }
}
