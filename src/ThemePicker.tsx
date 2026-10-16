import {useEffect,useState} from 'react';
import {desktop} from './playback';
import {useListening} from './listening';
import {applyTheme,cachedTheme,palettes,resolveTheme,type Mode} from './themes';
export function ThemeManager(){const settings=useListening();useEffect(()=>{if(desktop&&settings.ready)applyTheme(settings.preferences.theme);},[settings.ready,settings.preferences.theme]);return null;}
export function ThemePicker({settings}:{settings:ReturnType<typeof useListening>}){
 const [demoTheme,setDemoTheme]=useState(cachedTheme);const current=resolveTheme(desktop?settings.preferences.theme:demoTheme);
 const choose=(palette:string,mode:Mode)=>{const id=`${palette}-${mode}`;if(desktop)void settings.update({theme:id});else{applyTheme(id);setDemoTheme(id);}};
 return <details className="appearance"><summary>Appearance</summary><div className="appearance-panel"><strong>Make it yours</strong><div className="theme-modes" role="group" aria-label="Theme brightness">{(['dark','light'] as const).map(mode=><button key={mode} aria-pressed={current.mode===mode} disabled={desktop&&!settings.ready} onClick={()=>choose(current.palette.id,mode)}>{mode==='dark'?'☾ Dark':'☀ Light'}</button>)}</div><div className="theme-palettes" role="group" aria-label="Color palette">{palettes.map(palette=><button key={palette.id} className="theme-choice" aria-pressed={current.palette.id===palette.id} disabled={desktop&&!settings.ready} onClick={()=>choose(palette.id,current.mode)} title={palette.description}><span className="theme-swatches" aria-hidden="true"><i style={{background:palette[current.mode].bg}}/><i style={{background:palette[current.mode].surface}}/><i style={{background:palette[current.mode].accent}}/></span><span>{palette.name}</span>{current.palette.id===palette.id&&<span aria-hidden="true"> ✓</span>}</button>)}</div><p>Saved on this PC. Applies to the library and mini player.</p>{settings.error&&<p role="status">{settings.error}</p>}</div></details>;
}
