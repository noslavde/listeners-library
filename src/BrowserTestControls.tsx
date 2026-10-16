import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {Book} from './model';
import {desktop} from './playback';
import {safePlaybackMessage} from './streamLog';
export function BrowserTestControls({book,prepare}:{book?:Book;prepare:()=>Promise<void>}){
 const [url,setUrl]=useState(''),[busy,setBusy]=useState(false),[status,setStatus]=useState('');
 useEffect(()=>{setUrl('');setStatus('');return()=>{if(desktop)void invoke('browser_test_stop').catch(()=>{});};},[book?.asin]);
 const create=async()=>{if(!book)return;setBusy(true);try{await prepare();const result=await invoke<{url:string}>('browser_test_start',{asin:book.asin});setUrl(result.url);setStatus('Link ready · open it within two minutes. The test connection lasts 20 minutes.');}catch(error){setStatus(safePlaybackMessage(error));}finally{setBusy(false);}};
 return <section className="browser-test-controls"><strong>Compare playback in Edge or Chrome</strong><p>Tests this selected title with the same custom player and account connection. Keep Tauri running.</p><button disabled={!desktop||busy||!book||book.asin.startsWith('DEMO')} onClick={()=>void create()}>Create browser test link</button>{url&&<><button onClick={async()=>{try{await invoke('browser_test_open');setStatus('Opened the default browser · choose Load browser playback there');}catch(error){setStatus(safePlaybackMessage(error));}}}>Open default browser</button><button onClick={async()=>{try{await navigator.clipboard.writeText(url);setStatus('Copied · paste into Edge or Chrome. This pairing link works once.');}catch{setStatus('Select the test link below and copy it manually');}}}>Copy test link</button><input readOnly aria-label="Temporary browser test link" value={url}/><button onClick={async()=>{await invoke('browser_test_stop');setUrl('');setStatus('Browser comparison connection stopped');}}>Stop browser test</button></>}<p role="status">{status}</p></section>;
}
