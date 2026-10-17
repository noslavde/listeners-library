import { spawnSync } from 'node:child_process';
if (process.platform !== 'darwin') throw new Error('A macOS host is required.');
const target = process.env.LL_MAC_TARGET ?? (process.arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin');
if (!['aarch64-apple-darwin','x86_64-apple-darwin'].includes(target)) throw new Error('Unsupported Mac target');
const env = {...process.env, VITE_APP_CHANNEL: 'macos-experimental', LL_NOTICES_TARGET: target};
for (const script of ['build','notices','check:release']) {
  const result = spawnSync('npm',['run',script],{env,stdio:'inherit'});
  if (result.error || result.status !== 0) process.exit(result.status ?? 1);
}
