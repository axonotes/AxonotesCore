import {template} from "@axonotes/axogen";
import {config} from "../../config";

export const dockerComposeTarget = template({
  path: "axonotes-storage/docker-compose.yml",
  template: "axogen/templates/storage/docker-compose.yml.njk",
  engine: "nunjucks",
  variables: {
    storage: config.storage || {},
  },
  backup: true,
  generate_meta: false,
});
