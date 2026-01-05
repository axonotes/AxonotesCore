import {cmd, group, liveExec} from "@axonotes/axogen";
import {throwIfToolMissing} from "../utils/tool-detection";
import {ensureStorageConfig, generateDockerCompose} from "./shared/storage";
import * as z from "zod";

export const storageCmd = group({
  help: "Commands for managing the axonotes-storage service",
  commands: {
    dev: cmd({
      help: "Start the storage service in development mode",
      options: {
        rebuild: z
          .boolean()
          .default(false)
          .describe("Force rebuild of Docker images"),
      },
      exec: async (context) => {
        await throwIfToolMissing(
          "Docker",
          "docker",
          "--version",
          "https://docs.docker.com/get-docker/"
        );
        await throwIfToolMissing(
          "Docker Compose",
          "docker",
          "compose version",
          "https://docs.docker.com/compose/install/"
        );

        await ensureStorageConfig();
        await generateDockerCompose();

        const buildFlag = context.options.rebuild ? "--build" : "";
        await liveExec(`docker compose up ${buildFlag} -d`, {
          cwd: "axonotes-storage",
          outputPrefix: "STORAGE",
        });

        const {config} = await import("../config");
        const apiPort = config.storage?.api_port || 8081;
        const minioConsolePort = config.storage?.minio_console_port || 9001;

        console.log("\nStorage service started");
        console.log(`API: http://localhost:${apiPort}`);
        console.log(`Swagger UI: http://localhost:${apiPort}/swagger-ui`);
        console.log(`MinIO Console: http://localhost:${minioConsolePort}`);
      },
    }),

    stop: cmd({
      help: "Stop the storage service",
      exec: async () => {
        await throwIfToolMissing(
          "Docker",
          "docker",
          "--version",
          "https://docs.docker.com/get-docker/"
        );

        await liveExec("docker compose down", {
          cwd: "axonotes-storage",
          outputPrefix: "STORAGE",
        });
      },
    }),

    restart: cmd({
      help: "Restart the storage service",
      options: {
        rebuild: z
          .boolean()
          .default(false)
          .describe("Force rebuild of Docker images"),
      },
      exec: async (context) => {
        await throwIfToolMissing(
          "Docker",
          "docker",
          "--version",
          "https://docs.docker.com/get-docker/"
        );

        await liveExec("docker compose down", {
          cwd: "axonotes-storage",
          outputPrefix: "STORAGE",
        });

        await generateDockerCompose();

        const buildFlag = context.options.rebuild ? "--build" : "";
        await liveExec(`docker compose up ${buildFlag} -d`, {
          cwd: "axonotes-storage",
          outputPrefix: "STORAGE",
        });

        const {config} = await import("../config");
        const apiPort = config.storage?.api_port || 8081;
        const minioConsolePort = config.storage?.minio_console_port || 9001;

        console.log("\nStorage service restarted");
        console.log(`API: http://localhost:${apiPort}`);
        console.log(`Swagger UI: http://localhost:${apiPort}/swagger-ui`);
        console.log(`MinIO Console: http://localhost:${minioConsolePort}`);
      },
    }),

    logs: cmd({
      help: "View storage service logs",
      options: {
        follow: z.boolean().default(false).describe("Follow log output"),
        service: z
          .string()
          .optional()
          .describe(
            "Show logs for specific service (axonotes-storage, minio, minio-init)"
          ),
        tail: z.number().default(100).describe("Number of lines to show"),
      },
      exec: async (context) => {
        const followFlag = context.options.follow ? "-f" : "";
        const tailFlag = `--tail=${context.options.tail}`;
        const service = context.options.service || "";

        const flags = [followFlag, tailFlag, service].filter(Boolean).join(" ");

        await liveExec(`docker compose logs ${flags}`, {
          cwd: "axonotes-storage",
          outputPrefix: "STORAGE",
        });
      },
    }),

    status: cmd({
      help: "Check the status of the storage service",
      exec: async () => {
        await liveExec("docker compose ps", {
          cwd: "axonotes-storage",
          outputPrefix: "STORAGE",
        });
      },
    }),

    build: cmd({
      help: "Build the storage service Docker images",
      exec: async () => {
        await throwIfToolMissing(
          "Docker",
          "docker",
          "--version",
          "https://docs.docker.com/get-docker/"
        );

        await generateDockerCompose();

        await liveExec("docker compose build", {
          cwd: "axonotes-storage",
          outputPrefix: "STORAGE",
        });
      },
    }),

    test: cmd({
      help: "Run storage service tests",
      options: {
        integration: z
          .boolean()
          .default(false)
          .describe("Run integration tests (requires service to be running)"),
        bench: z
          .boolean()
          .default(false)
          .describe("Run benchmarks (requires service to be running)"),
      },
      exec: async (context) => {
        await throwIfToolMissing(
          "Cargo",
          "cargo",
          "--version",
          "https://rustup.rs/"
        );

        if (context.options.bench) {
          await liveExec("cargo bench", {
            cwd: "axonotes-storage",
            outputPrefix: "STORAGE-BENCH",
          });
        } else if (context.options.integration) {
          console.log(
            "Warning: Make sure the storage service is running before running integration tests"
          );
          await liveExec("cargo test --test integration_tests", {
            cwd: "axonotes-storage",
            outputPrefix: "STORAGE-TEST",
          });
        } else {
          await liveExec("cargo test", {
            cwd: "axonotes-storage",
            outputPrefix: "STORAGE-TEST",
          });
        }
      },
    }),

    clean: cmd({
      help: "Clean storage service data and volumes",
      options: {
        volumes: z
          .boolean()
          .default(false)
          .describe("Also remove Docker volumes (WARNING: deletes all data)"),
      },
      exec: async (context) => {
        const volumeFlag = context.options.volumes ? "-v" : "";
        await liveExec(`docker compose down ${volumeFlag}`, {
          cwd: "axonotes-storage",
          outputPrefix: "STORAGE",
        });

        if (context.options.volumes) {
          console.log("Warning: Docker volumes removed - all data deleted");
        }
      },
    }),
  },
});
