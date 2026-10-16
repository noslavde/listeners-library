import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {desktop} from './playback';
export function shortcutAction(event:{key:string;ctrlKey:boolean;altKey:boolean;metaKey:boolean;shiftKey:boolean;repeat:boolean;defaultPrevented:boolean},interactive:boolean){
 if(interactive||event.defaultPrevented||event.repeat||event.ctrlKey||event.altKey||event.metaKey||event.shiftKey)return null;
 return ({' ':'toggle',ArrowLeft:'back',ArrowRight:'forward',b:'bookmark',m:'mini'} as Record<string,string>)[event.key]??null;
}
export function usePlayerShortcuts(onError:(message:string)=>void){
 useEffect(()=>{if(!desktop)return;const handle=(e:KeyboardEvent)=>{const target=e.target instanceof Element?e.target:null;const interactive=!!target?.closest('input,textarea,select,button,a,[contenteditable]:not([contenteditable="false"]),[role="textbox"],[role="button"],[role="slider"],summary');const action=shortcutAction(e,interactive);if(!action)return;e.preventDefault();void invoke('player_shortcut',{action}).catch(e=>onError(String(e)));};window.addEventListener('keydown',handle);return()=>window.removeEventListener('keydown',handle);},[onError]);
}
type Status={global:string[];unavailable:string[];mediaKeys:boolean};
export function ShortcutHelp(){
 const [status,setStatus]=useState<Status>();
 useEffect(()=>{if(!desktop)return;let dead=false,stop:(()=>void)|undefined;invoke<Status>('shortcut_status').then(v=>{if(!dead)setStatus(v);}).catch(()=>{});listen<Status>('shortcut-status',e=>{if(!dead)setStatus(e.payload);}).then(f=>{if(dead)f();else stop=f;});return()=>{dead=true;stop?.();};},[]);
 if(!desktop)return null;
 return <details className="shortcut-help"><summary>Keyboard & media keys</summary><p>In the app: Space = play/pause; ← / → = skip 15 seconds; B = bookmark; M = mini player / return to library. These leave typing and focused controls alone.</p><p>Anywhere: Ctrl+Alt+Space, Ctrl+Alt+← / →, Ctrl+Alt+B, Ctrl+Alt+M.</p><p>Headset/media keys: play/pause; previous/next skip 15 seconds within the book.</p>{status&&<><p>{status.mediaKeys?'Windows media controls connected. Choose this app in Windows’ media controls if another player is active.':'Windows media controls could not connect; keyboard shortcuts are still available.'}</p>{status.unavailable.length>0&&<p role="status">Unavailable shortcuts (possibly used by another app): {status.unavailable.join(', ')}</p>}</>}</details>;
}
