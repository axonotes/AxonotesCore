import {defineConfig} from "@axonotes/axogen";
import {appConfigTarget} from "./axogen/targets/app/config-rs";
import {installCmd} from "./axogen/commands/install";
import {devCmd} from "./axogen/commands/dev";
import {fmtCmd} from "./axogen/commands/fmt";
import {stdbCmd} from "./axogen/commands/stdb";

export default defineConfig({
  targets: {
    rust_config: appConfigTarget,
  },
  commands: {
    install: installCmd,
    dev: devCmd,
    fmt: fmtCmd,
    stdb: stdbCmd,
  },
});
