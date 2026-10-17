import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const version = JSON.parse(readFileSync('package.json','utf8')).version;
const base = JSON.parse(readFileSync('src-tauri/tauri.conf.json','utf8'));
assert.match(version, /^\d+\.\d+\.\d+$/);
assert.equal(base.version, version, 'Frontend and Tauri versions must match');
const manifest = readFileSync('src-tauri/Cargo.toml','utf8').split('[build-dependencies]')[0];
assert.equal(manifest.match(/^version\s*=\s*"([^"]+)"/m)?.[1], version, 'Native version must match');
const ref = process.env.GITHUB_REF ?? '';
if (ref.startsWith('refs/tags/')) {
  const prefix = `refs/tags/macos-v${version}-alpha.`;
  assert(ref.startsWith(prefix) && /^[1-9]\d*$/.test(ref.slice(prefix.length)), 'Use a matching macos-vVERSION-alpha.NUMBER tag');
}
console.log(`Mac release versions match ${version}.`);
