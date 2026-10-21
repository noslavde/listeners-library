/**
 * Synthetic release-workflow regression harness using a stub GitHub CLI.
 * Exercises draft/published asset handling and checksum policy without
 * publishing a real release.
 */

import assert from "node:assert/strict";
import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  mkdtempSync,
  existsSync,
} from "node:fs";
import { resolve, join } from "node:path";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";

// Exercise the actual inline release step with a fake gh command: no network,
// real credentials, release creation or uploads are used by these checks.
const root = resolve(import.meta.dirname, "..");
const yaml = readFileSync(join(root, ".github/workflows/windows.yml"), "utf8");
const step = yaml.slice(
  yaml.indexOf("      - name: Attach installer or create draft test release"),
);
assert(step.includes("        run: |\n"));
const script = step
  .split("        run: |\n")[1]
  .split("\n")
  .map((line) => (line.startsWith("          ") ? line.slice(10) : line))
  .join("\n");
const bash =
  process.platform === "win32"
    ? join(process.env.ProgramFiles ?? "C:/Program Files", "Git/bin/bash.exe")
    : "bash";
const scratch = join(root, "work");
mkdirSync(scratch, { recursive: true });
const syntax = spawnSync(bash, ["-n"], { input: script, encoding: "utf8" });
assert.equal(syntax.status, 0, syntax.stderr);
const mock = [
  "gh() {",
  '  case "$1 $2" in',
  '    "release view")',
  '      [ "$MOCK_EXISTS" = true ] || return 1',
  '      if [ "$5" = assets ]; then printf "%s\\n" "$MOCK_ASSETS"; else printf "{}\\n"; fi ;;',
  '    "release upload") printf "upload:%s\\n" "$4" >> calls.txt ;;',
  '    "release create") printf "create:%s\\n" "$3" >> calls.txt ;;',
  '    "release download") mkdir -p existing; cp published.exe "existing/$5" ;;',
  '    *) echo "Unexpected gh call" >&2; return 2 ;;',
  "  esac",
  "}",
].join("\n");
function run({ exists, assets = "", existingOnly = "false", notes = false }) {
  const dir = mkdtempSync(join(scratch, "release-check-"));
  mkdirSync(join(dir, "deployment"));
  writeFileSync(join(dir, "deployment/Example-Setup.exe"), "fresh build");
  writeFileSync(join(dir, "deployment/SHA256SUMS.txt"), "fresh checksum");
  writeFileSync(join(dir, "deployment/PRIVACY.md"), "privacy guide");
  if (notes)
    writeFileSync(
      join(dir, "deployment/RELEASE_NOTES.md"),
      "Beta release notes",
    );
  writeFileSync(join(dir, "published.exe"), "existing published installer");
  const result = spawnSync(
    bash,
    ["--noprofile", "--norc", "-c", mock + "\n" + script],
    {
      cwd: dir,
      encoding: "utf8",
      env: {
        ...process.env,
        GH_TOKEN: "synthetic-test-token",
        GH_REPO: "example/test",
        RELEASE_TAG: "release_1",
        EXISTING_RELEASE_ONLY: existingOnly,
        MOCK_EXISTS: String(exists),
        MOCK_ASSETS: assets,
      },
    },
  );
  return {
    ...result,
    dir,
    calls: existsSync(join(dir, "calls.txt"))
      ? readFileSync(join(dir, "calls.txt"), "utf8")
      : "",
  };
}
let result = run({ exists: false, notes: true });
assert.equal(result.status, 0, result.stderr);
assert.match(result.calls, /create:release_1/);
result = run({ exists: false });
assert.equal(result.status, 0, result.stderr);
assert.match(result.calls, /create:release_1/);
result = run({ exists: false, existingOnly: "true" });
assert.notEqual(result.status, 0);
assert.equal(result.calls, "");
result = run({ exists: true, assets: "Example-Setup.exe" });
assert.equal(result.status, 0, result.stderr);
assert(!result.calls.includes("upload:deployment/Example-Setup.exe"));
assert(result.calls.includes("upload:deployment/PRIVACY.md"));
assert(result.calls.includes("upload:deployment/SHA256SUMS.txt"));
const checksum = createHash("sha256")
  .update("existing published installer")
  .digest("hex");
assert.match(
  readFileSync(join(result.dir, "deployment/SHA256SUMS.txt"), "utf8"),
  new RegExp("^" + checksum + " [ *]Example-Setup\\.exe"),
);
result = run({
  exists: true,
  assets: "Example-Setup.exe\nSHA256SUMS.txt\nPRIVACY.md",
});
assert.equal(result.status, 0, result.stderr);
assert.equal(result.calls, "");
console.log(
  "Release workflow checks passed: shell syntax, draft creation, missing-release failure, missing assets, published-installer checksum and safe reruns.",
);
