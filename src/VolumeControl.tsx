import {useEffect,useId,useRef,useState} from 'react';
export function VolumeControl({value=1,onChange,compact=false}:{value:number;onChange:(value:number)=>Promise<unknown>;compact?:boolean}){
 const normalized=Number.isFinite(value)?Math.max(0,Math.min(1,value)):1;
 const [draft,setDraft]=useState(normalized),[error,setError]=useState('');const ticks=useId(),timer=useRef<ReturnType<typeof setTimeout>|undefined>(undefined),callback=useRef(onChange),lastSent=useRef(normalized);callback.current=onChange;
 useEffect(()=>{setDraft(normalized);lastSent.current=normalized;},[normalized]);
 useEffect(()=>()=>clearTimeout(timer.current),[]);
 const commit=(volume:number,force=false)=>{clearTimeout(timer.current);if(!force&&volume===lastSent.current)return;lastSent.current=volume;void callback.current(volume).then(()=>setError('')).catch(e=>{setError(String(e));if(lastSent.current===volume){lastSent.current=normalized;setDraft(normalized);}});};
 const change=(volume:number)=>{setDraft(volume);clearTimeout(timer.current);timer.current=setTimeout(()=>commit(volume),120);};
 const percent=Math.round(draft*100);
 return <div className={compact?'volume-control volume-compact':'volume-control'}><div className="volume-heading"><label htmlFor={ticks+'-volume'}>{compact?'Volume':'App volume'}</label><output htmlFor={ticks+'-volume'}>{percent}%</output><button title="Reset app volume to 100%" onClick={()=>{setDraft(1);commit(1,true);}}>100%</button></div><input id={ticks+'-volume'} aria-label="App volume" aria-valuetext={`${percent}%`} type="range" min="0" max="100" step="1" list={ticks} value={percent} onChange={e=>change(Number(e.target.value)/100)} onPointerUp={()=>commit(draft)} onBlur={()=>commit(draft)}/><datalist id={ticks}><option value="0"/><option value="50"/><option value="100"/></datalist><div className="volume-scale" aria-hidden="true"><span>0%</span><span>50%</span><span>100%</span></div>{error&&<p role="status">{error}</p>}</div>;
}
