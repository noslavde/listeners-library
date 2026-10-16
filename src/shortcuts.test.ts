import {describe,it,expect,vi} from 'vitest';
vi.stubGlobal('window',{});
const {shortcutAction}=await import('./shortcuts');
const event=(key:string)=>({key,ctrlKey:false,altKey:false,metaKey:false,shiftKey:false,repeat:false,defaultPrevented:false});
describe('focused player shortcuts',()=>{
 it('maps transport, bookmark and mini-player keys',()=>{expect(shortcutAction(event(' '),false)).toBe('toggle');expect(shortcutAction(event('ArrowLeft'),false)).toBe('back');expect(shortcutAction(event('ArrowRight'),false)).toBe('forward');expect(shortcutAction(event('b'),false)).toBe('bookmark');expect(shortcutAction(event('m'),false)).toBe('mini');});
 it('does not steal typing, focused controls, modifiers or repeat events',()=>{expect(shortcutAction(event(' '),true)).toBe(null);for(const field of ['ctrlKey','altKey','metaKey','shiftKey','repeat','defaultPrevented'])expect(shortcutAction({...event(' '),[field]:true},false)).toBe(null);expect(shortcutAction(event('z'),false)).toBe(null);});
});
