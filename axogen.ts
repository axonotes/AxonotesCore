import {defineConfig} from "@axonotes/axogen";
import {appConfigTarget} from "./axogen/targets/app/config-rs";
import {dockerComposeTarget} from "./axogen/targets/storage/docker-compose";
import {installCmd} from "./axogen/commands/install";
import {devCmd} from "./axogen/commands/dev";
import {dev2Cmd} from "./axogen/commands/dev2";
import {fmtCmd} from "./axogen/commands/fmt";
import {stdbCmd} from "./axogen/commands/stdb";
import {testCmd} from "./axogen/commands/test";
import {storageCmd} from "./axogen/commands/storage";

export default defineConfig({
  targets: {
    rust_config: appConfigTarget,
    storage_docker_compose: dockerComposeTarget,
  },
  commands: {
    install: installCmd,
    dev: devCmd,
    dev2: dev2Cmd,
    fmt: fmtCmd,
    stdb: stdbCmd,
    test: testCmd,
    storage: storageCmd,
  },
});
