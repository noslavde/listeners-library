/**
 * Windows native command wrapper. Selects the verified local Perl toolchain
 * when available and forwards to Cargo tests or Tauri dev/NSIS builds without
 * embedding account configuration.
 */

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
const root = resolve(import.meta.dirname, "..");
const env = { ...process.env };
const portable = resolve(root, "work/build-tools/strawberry/perl/bin/perl.exe");
if (!env.OPENSSL_SRC_PERL && existsSync(portable))
  env.OPENSSL_SRC_PERL = portable;
env.PERL_BADLANG = "0";
const mode = process.argv[2];
const command = mode === "test" ? "cargo" : process.execPath;
const args =
  mode === "test"
    ? [
        "test",
        "--manifest-path",
        "src-tauri/Cargo.toml",
        ...process.argv.slice(3),
      ]
    : [
        resolve(root, "node_modules/@tauri-apps/cli/tauri.js"),
        ...(mode === "dev"
          ? ["dev", "--config", "src-tauri/tauri.debug.conf.json"]
          : ["build", "--bundles", "nsis"]),
        ...process.argv.slice(3),
      ];
const result = spawnSync(command, args, { cwd: root, env, stdio: "inherit" });
if (result.error)
  console.error(
    "Could not start native build. Check the documented Windows build prerequisites.",
  );
process.exit(result.status ?? 1);
