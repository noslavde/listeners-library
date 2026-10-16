import {afterEach,describe,expect,it,vi} from 'vitest';
import type shaka from 'shaka-player';
import {demoBooks} from './model';
import {dispatchDirectCommand} from './directCommands';

const mock=vi.hoisted(()=>({
 invoke:vi.fn(),plugins:new Map<string,shaka.extern.SchemePlugin>(),player:null as EventTarget|null,
 configuration:null as {drm:{advanced:Record<string,{serverCertificate:Uint8Array}>}}|null,
 play:vi.fn(async()=>{}),pause:vi.fn(),
}));
vi.mock('@tauri-apps/api/core',()=>({invoke:mock.invoke}));
vi.mock('@tauri-apps/api/event',()=>({listen:async()=>()=>{}}));
vi.mock('shaka-player',()=>({default:{
 polyfill:{installAll(){}},
 Player:class extends EventTarget{
  constructor(){super();mock.player=this;}
  static isBrowserSupported(){return true;}
  async attach(){} configure(config:typeof mock.configuration){mock.configuration=config;} async load(){} async destroy(){}
  getKeyStatuses(){return {};}
  keySystem(){return 'com.widevine.alpha';}
 },
 net:{NetworkingEngine:{RequestType:{LICENSE:2},registerScheme:(scheme:string,plugin:shaka.extern.SchemePlugin)=>mock.plugins.set(scheme,plugin)}},
 util:{AbortableOperation:class{constructor(public promise:Promise<unknown>) {}}},
}}));
import {StreamEngine} from './streamEngine';

let engine:StreamEngine|undefined;
afterEach(async()=>{await engine?.destroy();engine=undefined;mock.invoke.mockReset();mock.play.mockClear();mock.pause.mockClear();});
async function setup(){
 mock.invoke.mockImplementation(async(command:string)=>command==='api_stream_start'?{id:'test-session',manifest:'audible-media:manifest',resume:100,certificate:[5,6]}:command==='api_stream_license'?[8,9]:undefined);
 const audio=Object.assign(new EventTarget(),{currentTime:100,duration:1000,paused:true,playbackRate:1,readyState:1,volume:1,muted:false,seekable:{length:0},buffered:{length:0},pause:mock.pause,play:mock.play}) as unknown as HTMLAudioElement;
 const status=vi.fn();
 engine=new StreamEngine(audio,demoBooks[0],status);
 await engine.load();
 return status;
}
function request(keySystem:string){return {body:new Uint8Array([99,1,2,99]).subarray(1,3),headers:{},drmInfo:{keySystem}} as unknown as shaka.extern.Request;}
describe('protected streaming license transport',()=>{
 it('preloads the provider service certificate before media playback',async()=>{
  await setup();
  expect(Array.from(mock.configuration!.drm.advanced['com.widevine.alpha'].serverCertificate)).toEqual([5,6]);
 });
 it('does not send the Widevine two-byte certificate request to the book license API',async()=>{
  const status=await setup();
  const certificateRequest={...request('com.widevine.alpha'),body:new Uint8Array([8,4])};
  await expect(mock.plugins.get('audible-license')!('audible-license:test-session',certificateRequest,2,vi.fn(),vi.fn(),{}).promise).rejects.toThrow('service certificate');
  expect(mock.invoke.mock.calls.some(([command])=>command==='api_stream_license')).toBe(false);
  expect(status).toHaveBeenCalledWith(expect.stringContaining('service certificate'));
 });
 it('routes an HTTPS license challenge through the authenticated license endpoint, preserving only its actual bytes',async()=>{
  const status=await setup();
  const operation=mock.plugins.get('https')!('https://license.example.test/private',request('com.widevine.alpha'),2,vi.fn(),vi.fn(),{});
  const response=await operation.promise;
  expect(mock.invoke).toHaveBeenCalledWith('api_stream_license',{id:'test-session',challenge:[1,2]});
  expect(mock.invoke.mock.calls.some(([command])=>command==='api_stream_fetch')).toBe(false);
  expect(Array.from(new Uint8Array(response.data as ArrayBuffer))).toEqual([8,9]);
  expect(status).toHaveBeenCalledWith('Platform license request sent');
  expect(status).toHaveBeenCalledWith('Platform license response received');
 });
 it('reports a rejected license separately from a missing license request',async()=>{
  const status=await setup();
  mock.invoke.mockRejectedValueOnce('Protected streaming API returned HTTP 403');
  await expect(mock.plugins.get('audible-license')!('audible-license:test-session',request('com.widevine.alpha'),2,vi.fn(),vi.fn(),{}).promise).rejects.toThrow('HTTP 403');
  expect(status).toHaveBeenCalledWith('Platform license request failed');
  mock.player?.dispatchEvent(Object.assign(new Event('error'),{detail:{code:6007,data:['https://private.test/?token=secret']}}));
  expect(status).toHaveBeenLastCalledWith(expect.stringContaining('HTTP 403'));
  expect(status.mock.lastCall?.[0]).not.toContain('secret');
  expect(mock.pause).toHaveBeenCalled();
  dispatchDirectCommand('play',0);
  expect(mock.play).not.toHaveBeenCalled();
  expect(status).toHaveBeenLastCalledWith(expect.stringContaining('Playback stopped'));
 });
 it('never submits another DRM system challenge as a Widevine license request',async()=>{
  await setup();
  await expect(mock.plugins.get('https')!('https://license.example.test',request('com.microsoft.playready'),2,vi.fn(),vi.fn(),{}).promise).rejects.toThrow('only supports Widevine');
  expect(mock.invoke.mock.calls.some(([command])=>command==='api_stream_license'||command==='api_stream_fetch')).toBe(false);
 });
});
