import {cmd, group} from "@axonotes/axogen";
import {
  generateRustBindings,
  publishSpacetimeModule,
  startSpacetimeDBServer,
} from "./shared/stdb";
import * as z from "zod";

export const stdbCmd = group({
  help: "Commands related to spacetimedb",
  commands: {
    generate: cmd({
      help: "Generate spacetimedb bindings",
      exec: async () => {
        await generateRustBindings();
      },
    }),
    dev: cmd({
      help: "Start the spacetimedb development server",
      options: {
        c: z
          .boolean()
          .default(false)
          .describe(
            "Whether to clear existing data before publishing the module"
          ),
        deleteData: z
          .boolean()
          .default(false)
          .describe(
            "Whether to clear existing data before publishing the module (alias for -c)"
          ),
      },
      exec: async (context) => {
        // start spacetimedb server in async
        const startPromise = startSpacetimeDBServer();

        setTimeout(async () => {
          // publish module to local server
          await publishSpacetimeModule(
            context.options.c || context.options.deleteData
          );

          console.log("========== Server is now ready for use. ==========");
        }, 2000); // wait 2 seconds for server to start

        await startPromise;
      },
    }),
  },
});
