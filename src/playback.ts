import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {Book,Snapshot} from './model';
import {dispatchDirectCommand} from './directCommands';
export const desktop=!!(window as unknown as {__TAURI_INTERNALS__?:unknown}).__TAURI_INTERNALS__;
export type Action='play'|'pause'|'seek'|'speed'|'volume'|'pauseChapter'|'rewind'|'recover';
export interface PlaybackAdapter{open(book:Book):Promise<void>;command(action:Action,value?:number):Promise<void>;subscribe(fn:(s:Snapshot)=>void):Promise<()=>void>}
export class ApiPlaybackAdapter implements PlaybackAdapter{
 open(book:Book){return invoke<void>('select_api_title',{asin:book.asin});}
 command(action:Action,value=0){if(dispatchDirectCommand(action,value))return Promise.resolve();return invoke<void>('api_player_command',{action,value});}
 subscribe(fn:(s:Snapshot)=>void){return new OfficialAdapter().subscribe(fn);}
}
export class OfficialAdapter implements PlaybackAdapter{
 open(book:Book){return invoke<void>('open_player',{asin:book.asin});}
 chapter(position:number,title:string){return invoke<void>('select_chapter',{position,title});}
 command(action:Action,value=0){return invoke<void>('player_command',{action,value});}
 async subscribe(fn:(s:Snapshot)=>void){let last=0;let current:Snapshot={position:0,duration:0,speed:1,paused:true,available:false};const stop=await listen<Snapshot>('player-state',e=>{const p=e.payload;if(!p||![p.position,p.duration,p.speed].every(Number.isFinite)||p.position<0||p.duration<0||p.speed<.5||p.speed>3||typeof p.paused!=='boolean'||typeof p.available!=='boolean')return;last=Date.now();current=p;fn(p);});const timer=setInterval(()=>{if(Date.now()-last>2500)fn({...current,available:false,paused:true});},1000);return()=>{stop();clearInterval(timer);};}
}
export class DemoAdapter implements PlaybackAdapter{
 private s:Snapshot={position:0,duration:0,speed:1,volume:1,paused:true,available:true};private chapterEnd=Infinity;private pauseChapter=false;private pausedAt=Date.now();private rewind=5;private chapters:number[]=[];
 async open(b:Book){this.chapters=b.chapters;this.s={...this.s,position:0,duration:b.duration,paused:true};}
 async command(a:Action,v=0){if(a==='play'){if(Date.now()-this.pausedAt>60000)this.s.position=Math.max(0,this.s.position-this.rewind);this.chapterEnd=this.chapters.find(c=>c>this.s.position)??this.s.duration;this.s.paused=false;}if(a==='pause'){this.s.paused=true;this.pausedAt=Date.now();}if(a==='seek')this.s.position=Math.max(0,Math.min(this.s.duration,v));if(a==='speed')this.s.speed=v;if(a==='volume')this.s.volume=Math.max(0,Math.min(1,v));if(a==='pauseChapter')this.pauseChapter=!!v;if(a==='rewind')this.rewind=v;}
 async subscribe(fn:(s:Snapshot)=>void){const timer=setInterval(()=>{if(!this.s.paused){this.s.position=Math.min(this.s.duration,this.s.position+this.s.speed*.25);if(this.s.position>=this.s.duration||(this.pauseChapter&&this.s.position>=this.chapterEnd)){this.s.paused=true;this.pausedAt=Date.now();}}fn({...this.s});},250);return ()=>clearInterval(timer);}
}
