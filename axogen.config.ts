import {defineConfig} from "@axonotes/axogen";
import {commands} from "./config/commands";
import {dashboard} from "./config/targets/dashboard";

export default defineConfig({
    targets: {
        dashboard,
    },
    commands,
});
