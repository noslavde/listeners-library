import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';
import {describe,it,expect} from 'vitest';
const source=readFileSync(new URL('../src-tauri/src/volume.js',import.meta.url),'utf8');
function fixture(){
 let contexts=0;const window={AudioContext:class {constructor(){contexts++;throw Error('Not needed');}}} as unknown as {__audibleVolume:{apply:(m:unknown,v:number,explicit?:boolean)=>{volume:number};status:(m:unknown)=>{volume:number}}};
 const location={protocol:'https:',hostname:'www.audible.com',pathname:'/webplayer',href:'https://www.audible.com/webplayer?asin=B012345678'};
 runInNewContext(source,{window,location,WeakMap,Number});
 const media={volume:1,muted:false,mediaKeys:{},currentSrc:'https://cdn.example/audio'};
 return {controller:window.__audibleVolume,media,contexts:()=>contexts};
}
describe('app-only volume',()=>{
 it('defaults to unity and supports silence, attenuation and reset',()=>{const f=fixture();expect(f.controller.status(f.media).volume).toBe(1);f.controller.apply(f.media,0);expect(f.media.volume).toBe(0);f.controller.apply(f.media,.45);expect(f.media.volume).toBe(.45);f.controller.apply(f.media,1);expect(f.media.volume).toBe(1);});
 it('rejects values above 100% and other invalid values',()=>{const f=fixture();f.controller.apply(f.media,.4);for(const n of [-1,1.01,2,NaN,Infinity])f.controller.apply(f.media,n);expect(f.media.volume).toBe(.4);});
 it('uses normal volume for protected audio without creating an audio context',()=>{const f=fixture();f.controller.apply(f.media,.3);expect(f.media.volume).toBe(.3);expect(f.contexts()).toBe(0);});
 it('uses normal volume for cross-origin playback',()=>{const f=fixture();f.controller.apply(f.media,.65);expect(f.media.volume).toBe(.65);expect(f.contexts()).toBe(0);});
 it('unmutes only when an explicit nonzero volume is chosen',()=>{const f=fixture();f.media.muted=true;f.controller.apply(f.media,.5);expect(f.media.muted).toBe(true);f.controller.apply(f.media,.5,true);expect(f.media.muted).toBe(false);});
 it('keeps the last configured value separate for each media element',()=>{const f=fixture();f.controller.apply(f.media,.5);const next={...f.media,volume:1};f.controller.apply(next,.5);expect(next.volume).toBe(.5);});
 it('reports mute without changing the stored media level',()=>{const f=fixture();f.media.volume=.4;f.media.muted=true;expect(f.controller.status(f.media)).toEqual({volume:0});expect(f.media.volume).toBe(.4);});
});
