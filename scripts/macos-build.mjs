import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
if (process.platform !== 'darwin') throw new Error('Mac builds require macOS and Xcode command-line tools. Use the experimental GitHub workflow from Windows.');
const mode = process.argv[2] ?? 'build';
if (!['build','dev'].includes(mode)) throw new Error('Unknown mode');
const env = {...process.env,VITE_APP_CHANNEL:'macos-experimental'};
const args = [resolve('node_modules/@tauri-apps/cli/tauri.js'),mode,'--config','src-tauri/tauri.macos.conf.json',...process.argv.slice(3)];
const result = spawnSync(process.execPath,args,{env,stdio:'inherit'});
process.exit(result.status ?? 1);
