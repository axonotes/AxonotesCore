import {template} from "@axonotes/axogen";
import {config, OAuthSchema} from "../../config";

export const appConfigTarget = template({
  path: "axonotes-app/src-tauri/src/config.rs",
  template: "axogen/templates/app/config.rs.njk",
  engine: "nunjucks",
  schema: OAuthSchema,
  variables: config,
  backup: true,
  generate_meta: true,
});
