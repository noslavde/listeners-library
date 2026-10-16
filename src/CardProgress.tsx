import type {Book} from './model';
import type {CardPositions} from './useCardPositions';
function time(seconds:number){return `${Math.floor(seconds/3600)}:${String(Math.floor(seconds/60)%60).padStart(2,'0')}:${String(Math.floor(seconds)%60).padStart(2,'0')}`;}
export function CardProgress({book,saved,livePosition}:{book:Book;saved?:CardPositions[string];livePosition?:number}){
 const local=livePosition??saved?.local?.position,cloud=saved?.cloud?.position;
 const position=local??cloud;
 const percent=position!==undefined&&book.duration>0?Math.max(0,Math.min(100,position/book.duration*100)):null;
 return <div className="card-progress"><p>{position===undefined?'Last position unavailable':`Last position ${time(position)} · ${local!==undefined?(livePosition!==undefined?'this player':'local'):'Audible'}`}</p>{cloud!==undefined&&local!==undefined&&<p title={saved?.cloud?`Audible position last checked ${new Date(saved.cloud.updated*1000).toLocaleString()}`:undefined}>Audible {time(cloud)}</p>}{percent!==null&&<><progress aria-label={`${book.title} listening progress`} max={100} value={percent}/><small>{Math.round(percent)}% of book</small></>}</div>;
}
