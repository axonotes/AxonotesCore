import {liveExec} from "@axonotes/axogen";
import {throwIfToolMissing} from "../../utils/tool-detection";

export async function generateRustBindings() {
  await throwIfToolMissing("Cargo", "cargo", "--version", "https://rustup.rs/");
  await throwIfToolMissing(
    "Spacetime CLI",
    "spacetime",
    "--version",
    "https://spacetimedb.com/install"
  );

  await liveExec(
    "spacetime generate --lang rust --out-dir axonotes-app/src-tauri/src/stdb_bindings --project-path axonotes-stdb",
    {
      outputPrefix: "SPACETIME-RUST",
    }
  );
}

export async function startSpacetimeDBServer() {
  await throwIfToolMissing("Cargo", "cargo", "--version", "https://rustup.rs/");
  await throwIfToolMissing(
    "Spacetime CLI",
    "spacetime",
    "--version",
    "https://spacetimedb.com/install"
  );

  await liveExec("spacetime start", {
    outputPrefix: "SPACETIME-DB",
  });
}

export async function publishSpacetimeModule(deleteData: boolean = false) {
  await throwIfToolMissing("Cargo", "cargo", "--version", "https://rustup.rs/");
  await throwIfToolMissing(
    "Spacetime CLI",
    "spacetime",
    "--version",
    "https://spacetimedb.com/install"
  );

  const deleteFlag = deleteData ? "--delete-data" : "";
  const flags = [deleteFlag].join(" ").trim();

  await liveExec(
    "spacetime publish --server local --project-path axonotes-stdb axonotes " +
      flags,
    {
      outputPrefix: "SPACETIME-PUBLISH",
    }
  );
}
