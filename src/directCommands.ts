import type {Action} from './playback';

// Same-window controls must call play() before the click's activation is lost to IPC.
let handler:((action:Action,value:number)=>void)|undefined;
export function registerDirectCommands(next:(action:Action,value:number)=>void){
 handler=next;
 return()=>{if(handler===next)handler=undefined;};
}
export function dispatchDirectCommand(action:Action,value:number){
 if(!handler)return false;
 handler(action,value);
 return true;
}
export function playFailure(error:unknown){
 const name=error&&typeof error==='object'&&'name' in error?error.name:'';
 if(name==='NotAllowedError')return 'Playback permission denied · press Play on the direct audio controls';
 if(name==='NotSupportedError')return 'Audio could not start · the runtime rejected the audio format or protected playback configuration';
 if(name==='AbortError')return 'Playback start was interrupted · try Play again after loading or seeking finishes';
 return 'Audio could not start · check the playback status below';
}
