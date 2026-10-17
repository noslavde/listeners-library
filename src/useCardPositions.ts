import {useCallback,useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {desktop} from './playback';
type Position={position:number;updated:number};
export type CardPositions=Record<string,{local?:Position;cloud?:Position}>;
export function useCardPositions(asins:string){
 const [positions,setPositions]=useState<CardPositions>({}),[status,setStatus]=useState(''),[busy,setBusy]=useState(false);
 const lock=useRef(false),alive=useRef(true),epoch=useRef(0);
 const load=useCallback(async()=>{const version=epoch.current;const rows=await invoke<CardPositions>('list_card_positions');if(alive.current&&version===epoch.current)setPositions(rows);},[]);
 const refresh=useCallback(async()=>{if(!desktop||lock.current)return;const version=epoch.current;lock.current=true;setBusy(true);try{const result=await invoke<{updated:number;warnings:string[]}>('api_refresh_card_positions');if(version!==epoch.current)return;await load();if(alive.current&&version===epoch.current)setStatus(result.warnings.length?result.warnings.join('; '):`Audible positions checked · ${result.updated} titles updated`);}catch(e){if(alive.current&&version===epoch.current)setStatus(String(e));}finally{lock.current=false;if(alive.current)setBusy(false);}},[load]);
 useEffect(()=>{alive.current=true;const stops:(()=>void)[]=[];if(desktop)for(const event of ['account-signed-out','api-account-changed'])void listen(event,()=>{epoch.current++;setPositions({});setStatus('');}).then(f=>{if(alive.current)stops.push(f);else f();});return()=>{alive.current=false;epoch.current++;stops.forEach(f=>f());};},[]);
 useEffect(()=>{if(!desktop)return;let dead=false;load().catch(e=>{if(!dead)setStatus(String(e));});invoke<boolean>('api_status').then(connected=>{if(connected&&!dead)void refresh();}).catch(()=>{});return()=>{dead=true;};},[asins,load,refresh]);
 useEffect(()=>{if(!desktop)return;let dead=false,timer:ReturnType<typeof setTimeout>|undefined;const stops:(()=>void)[]=[];const keep=(f:()=>void)=>{if(dead)f();else stops.push(f);};listen('card-positions-changed',()=>{clearTimeout(timer);timer=setTimeout(()=>{load().catch(()=>{});},200);}).then(keep);listen('api-account-changed',()=>{invoke<boolean>('api_status').then(connected=>{if(connected&&!dead)void refresh();}).catch(()=>{});}).then(keep);return()=>{dead=true;clearTimeout(timer);stops.forEach(f=>f());};},[load,refresh]);
 return {positions,status,busy,refresh};
}
