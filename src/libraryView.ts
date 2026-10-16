import type {Book} from './model';
import type {CardPositions} from './useCardPositions';
export type Sort='title'|'author'|'series'|'recent'|'shortest'|'longest'|'progress';
export type Activity=Record<string,{listenedSeconds:number;lastListened:number;position:number;duration:number;hidden:boolean}>;
const compare=(a:string,b:string)=>a.localeCompare(b,undefined,{numeric:true,sensitivity:'base'});
export function sortedBooks(books:Book[],sort:Sort,activity:Activity,positions:CardPositions){
 const progress=(b:Book)=>b.duration>0?(positions[b.asin]?.local?.position??positions[b.asin]?.cloud?.position??activity[b.asin]?.position??0)/b.duration:0;
 return books.slice().sort((a,b)=>{
  let order=0;
  switch(sort){case 'author':order=compare(a.author,b.author);break;case 'series':order=(Number(!a.series)-Number(!b.series))||compare(a.series,b.series)||(a.volume-b.volume);break;case 'recent':order=(activity[b.asin]?.lastListened??0)-(activity[a.asin]?.lastListened??0);break;case 'shortest':order=(a.duration||Infinity)-(b.duration||Infinity);break;case 'longest':order=(b.duration||0)-(a.duration||0);break;case 'progress':order=progress(b)-progress(a);break;}
  return order||compare(a.title,b.title)||compare(a.asin,b.asin);
 });
}
export function continueListening(books:Book[],activity:Activity,now=Date.now()/1000){
 return books.filter(b=>{
  const a=activity[b.asin],duration=b.duration||a?.duration;if(!a||a.hidden||!duration||duration<=0)return false;
  const threshold=Math.max(30,Math.min(300,duration*.05));
  return a.listenedSeconds>=threshold&&a.lastListened>=now-30*86400&&a.lastListened<=now+60&&a.position>0&&a.position/duration<.98;
 }).sort((a,b)=>activity[b.asin].lastListened-activity[a.asin].lastListened||compare(a.title,b.title)).slice(0,6);
}
