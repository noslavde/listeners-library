import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {Book} from './model';
import {desktop} from './playback';
import {continueListening,type Activity} from './libraryView';
import {Cover} from './Cover';
export function useLibraryActivity(){
 const [activity,setActivity]=useState<Activity>({});
 useEffect(()=>{if(!desktop)return;let dead=false,stop:(()=>void)|undefined;const load=()=>invoke<Activity>('library_activity').then(v=>{if(!dead)setActivity(v);});load().catch(()=>{});listen('library-activity-changed',()=>{void load().catch(()=>{});}).then(f=>{if(dead)f();else stop=f;});const timer=setInterval(()=>{void load().catch(()=>{});},60000);return()=>{dead=true;stop?.();clearInterval(timer);};},[]);
 return activity;
}
export function ContinueShelf({books,activity,onListen,onStatus,onSetCurrent,pinBusy}:{books:Book[];activity:Activity;onListen:(book:Book)=>void;onStatus:(status:string)=>void;onSetCurrent?:(book:Book)=>void;pinBusy?:boolean}){
 const [dismissed,setDismissed]=useState<Book>();const shelf=continueListening(books,activity);
 const hide=async(b:Book,hidden:boolean)=>{try{await invoke('dismiss_continue',{asin:b.asin,hidden});setDismissed(hidden?b:undefined);}catch(e){onStatus(String(e));}};
 return <section className="continue-shelf"><div className="section-heading"><h2>Continue listening</h2><details><summary>How this works</summary><p>Books appear after five minutes of actual listening in this app, or 5% of a short book (at least 30 seconds). Brief tests, seeking and cloud refreshes don’t count. Books leave after 30 days without listening or at 98% complete. Up to six recent titles appear.</p><p>Removing a book hides it until you choose to listen to it again. History stays on this PC; listening before this update cannot be verified.</p></details></div>{!shelf.length&&<p className="empty-shelf">Your next read will appear here after some meaningful listening.</p>}<div className="shelf-books">{shelf.map(b=><article key={b.asin}><Cover book={b}/><div><h3>{b.title}</h3><p>{Math.round(activity[b.asin].position/(b.duration||activity[b.asin].duration)*100)}% listened · {new Date(activity[b.asin].lastListened*1000).toLocaleDateString()}</p><button className="primary-action" onClick={()=>onListen(b)}>Resume</button>{onSetCurrent&&<button disabled={pinBusy} onClick={()=>onSetCurrent(b)}>Set as current</button>}<button aria-label={`Remove ${b.title} from Continue listening`} onClick={()=>void hide(b,true)}>Remove</button></div></article>)}</div>{dismissed&&<p role="status">Removed {dismissed.title}. <button onClick={()=>void hide(dismissed,false)}>Undo</button></p>}</section>;
}
