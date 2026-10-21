/**
 * Release packaging assertions for the built frontend and native command
 * registration. Rejects development/private resources and requires experimental
 * native commands to remain immediately cfg-gated, independent of formatting
 * whitespace.
 */

import { readFile, readdir } from "node:fs/promises";
import assert from "node:assert/strict";
const config = JSON.parse(await readFile("src-tauri/tauri.conf.json", "utf8"));
assert(config.bundle.active, "Installer bundling must be enabled");
assert(
  config.app.windows.every((w) => w.devtools === false),
  "Release developer tools must be disabled",
);
assert.deepEqual(
  config.bundle.resources,
  {
    "resources/THIRD_PARTY_LICENSES.txt": "THIRD_PARTY_LICENSES.txt",
    "../LICENSE": "LICENSE.txt",
  },
  "Only reviewed resources may be packaged",
);
const files = await readdir("dist", { recursive: true });
assert(
  files.every(
    (f) =>
      !/(browser-test|mini-preview|\.map$|\.sqlite|\.db$|\.auth|\.env)/i.test(
        f,
      ),
  ),
  "Development or private files in frontend bundle",
);
const forbidden = [
  "api_stream_start",
  "api_stream_license",
  "api_prepare_upload",
  "api_commit_upload",
  "browser_test_start",
  "inspect_window",
  "mockIPC",
  "api_restore_previous_account",
];
for (const file of files.filter((f) => /\.(js|html)$/.test(f))) {
  const text = await readFile(`dist/${file}`, "utf8");
  for (const name of forbidden)
    assert(
      !text.includes(name),
      `Development command ${name} in shipped frontend`,
    );
}
const source = (await readFile("src-tauri/src/main.rs", "utf8")).replaceAll(
  "\r\n",
  "\n",
);
const handlerStart = source.search(
  /\.invoke_handler\(\s*tauri::generate_handler!\s*\[/,
);
assert(handlerStart >= 0, "Native command registration must be present");
const handler = source.slice(handlerStart);
assert(
  !handler.includes("api_restore_previous_account"),
  "Retired account restore command must not be exposed",
);
for (const command of [
  "inspect_window",
  "metadata::api_streaming_probe",
  "metadata::stream::api_stream_start",
  "metadata::stream::api_stream_fetch",
  "metadata::stream::api_stream_license",
  "metadata::stream::api_stream_state",
  "metadata::stream::api_player_command",
  "metadata::stream::api_stream_end",
  "metadata::browser_test::browser_test_start",
  "metadata::browser_test::browser_test_open",
  "metadata::browser_test::browser_test_stop",
  "desktop::select_api_title",
  "metadata::upload::api_prepare_upload",
  "metadata::upload::api_commit_upload",
  "metadata::upload::api_cancel_upload",
  "sync_visible_library",
  "import_library",
]) {
  assert.match(
    handler,
    new RegExp(String.raw`#\[cfg\(debug_assertions\)\]\s+${command}\s*,`),
    `Experimental native command must be debug-only: ${command}`,
  );
}
console.log(
  "Release checks passed: developer UI, experimental commands and packaged resources.",
);
