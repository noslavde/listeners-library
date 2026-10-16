import {useState} from 'react';
import type {Book} from './model';
export function safeCover(raw?:string){try{const u=new URL(raw??'');return u.protocol==='https:'&&!u.username&&!u.password&&(!u.port||u.port==='443')&&['m.media-amazon.com','images-na.ssl-images-amazon.com'].includes(u.hostname)&&u.pathname.startsWith('/images/I/')&&!u.search&&!u.hash?u.href:undefined;}catch{return undefined;}}
export function Cover({book}:{book:Book}){const [failed,setFailed]=useState('');const url=safeCover(book.coverUrl);return url&&url!==failed?<img className="book-cover" src={url} alt={`Cover of ${book.title}`} referrerPolicy="no-referrer" loading="lazy" onError={()=>setFailed(url)}/>:<div className="cover" aria-hidden="true">{book.title.slice(0,1)}</div>;}
