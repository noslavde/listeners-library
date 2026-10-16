import {describe,it,expect} from 'vitest';
import {chapterInfo,finishTime} from './playerInfo';
import {safeCover} from './Cover';
import type {Book,Snapshot} from './model';
const book:Book={asin:'B012345678',title:'Test',author:'A',series:'',volume:0,duration:600,chapters:[0,200,400],chapterDetails:[{title:'Opening',start:0,length:200},{title:'Arrival',start:200,length:200},{title:'Home',start:400,length:200}]};
const snapshot:Snapshot={available:true,paused:false,position:250,duration:600,speed:2};
describe('mini player information',()=>{
 it('shows named chapter progress and speed-adjusted remaining time',()=>{expect(chapterInfo(book,snapshot)).toMatchObject({title:'Arrival',position:50,duration:200,remaining:75,percent:25});expect(finishTime(snapshot,1000)).toBe(176000);});
 it('chooses the next chapter exactly at its boundary and clamps the final end',()=>{expect(chapterInfo(book,{...snapshot,position:400})?.title).toBe('Home');expect(chapterInfo(book,{...snapshot,position:600})).toMatchObject({remaining:0,percent:100});});
 it('does not invent chapter information when offsets are missing',()=>{expect(chapterInfo({...book,chapters:[],chapterDetails:[]},snapshot)).toBe(null);expect(chapterInfo(book,{...snapshot,available:false})).toBe(null);});
 it('permits only public cover image URLs without credentials or query strings',()=>{expect(safeCover('https://m.media-amazon.com/images/I/cover.jpg')).toBeTruthy();for(const url of ['https://evil.test/cover.jpg','https://m.media-amazon.com/images/I/a?token=secret','https://private@m.media-amazon.com/images/I/a','data:image/png;base64,a'])expect(safeCover(url)).toBeUndefined();});
});
