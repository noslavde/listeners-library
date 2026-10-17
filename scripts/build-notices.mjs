import {execFileSync} from 'node:child_process';
import {readFileSync,readdirSync,existsSync,statSync,mkdirSync,writeFileSync} from 'node:fs';
import {resolve,dirname,basename,sep} from 'node:path';
const root=resolve(import.meta.dirname,'..');
const metadata=JSON.parse(execFileSync('cargo',['metadata','--manifest-path',resolve(root,'src-tauri/Cargo.toml'),'--format-version','1','--filter-platform','x86_64-pc-windows-msvc','--offline'],{encoding:'utf8',maxBuffer:40*1024*1024}));
const nodes=new Map(metadata.resolve.nodes.map(n=>[n.id,n]));
const ids=new Set();function visit(id){if(ids.has(id))return;ids.add(id);for(const d of nodes.get(id)?.dependencies??[])visit(d);}visit(metadata.resolve.root);
function licenses(dir,extra){
 const names=readdirSync(dir).filter(n=>/^(licen[cs]e|copying|notice|unlicense)([._-]|$)/i.test(n));
 if(extra){const p=resolve(dir,extra);if(p.startsWith(dir+sep)&&existsSync(p))names.push(extra);}
 return [...new Set(names)].flatMap(n=>{const p=resolve(dir,n);if(!p.startsWith(dir+sep)||!existsSync(p)||!statSync(p).isFile()||statSync(p).size>512000)return [];return [`--- ${basename(n)} ---\n${readFileSync(p,'utf8')}`];});
}
const out=['Listener’s Library — third-party notices','This private test build is not affiliated with Audible or Amazon.','This file includes notices for frontend runtime components and the Windows native dependency graph (including build-time components).','Protocol reference attribution:',readFileSync(resolve(root,'docs/THIRD_PARTY_NOTICES.md'),'utf8')];
for(const pkg of metadata.packages.filter(p=>ids.has(p.id)&&p.source).sort((a,b)=>a.name.localeCompare(b.name))){const dir=dirname(pkg.manifest_path);out.push(`\n=== Rust: ${pkg.name} ${pkg.version} ===\nDeclared license: ${pkg.license??'See license text'}`, ...licenses(dir,pkg.license_file));if(pkg.name==='libsqlite3-sys')out.push(...licenses(dir,'sqlcipher/LICENSE'));if(pkg.name==='openssl-src')out.push(...licenses(dir,'openssl/LICENSE.txt'));}
for(const name of ['react','react-dom','scheduler','@tauri-apps/api']){const dir=resolve(root,'node_modules',name),p=JSON.parse(readFileSync(resolve(dir,'package.json'),'utf8'));out.push(`\n=== JavaScript: ${name} ${p.version} ===\nDeclared license: ${p.license??'See license text'}`,...licenses(dir));}
const target=resolve(root,'src-tauri/resources');mkdirSync(target,{recursive:true});writeFileSync(resolve(target,'THIRD_PARTY_LICENSES.txt'),out.join('\n\n'));
process.stdout.write(`Third-party notices generated for ${ids.size-1} native components and 4 frontend runtime packages.\n`);
