import {exec} from "@axonotes/axogen";
import * as fs from "fs";
import * as path from "path";

const STORAGE_DIR = path.join(process.cwd(), "axonotes-storage");
const CONFIG_FILE = path.join(STORAGE_DIR, "config.toml");
const EXAMPLE_CONFIG_FILE = path.join(STORAGE_DIR, "config.example.toml");

export async function ensureStorageConfig() {
  if (!fs.existsSync(CONFIG_FILE)) {
    console.log("Config file not found, copying from example...");

    if (!fs.existsSync(EXAMPLE_CONFIG_FILE)) {
      throw new Error(
        `Example config file not found at ${EXAMPLE_CONFIG_FILE}`
      );
    }

    fs.copyFileSync(EXAMPLE_CONFIG_FILE, CONFIG_FILE);
    console.log(`Created config file at ${CONFIG_FILE}`);
  }
}

export async function generateDockerCompose() {
  console.log("Generating docker-compose.yml from config...");
  await exec("axogen gen --target storage_docker_compose --quiet");
}
