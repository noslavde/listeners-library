// Reads only title metadata from the currently rendered official library.
(()=>{
 if(location.protocol!=='https:'||!['audible.com','www.audible.com'].includes(location.hostname)||!location.pathname.startsWith('/library'))return;
 const books=new Map();
 for(const a of document.querySelectorAll('a[href]')){
  const u=new URL(a.getAttribute('href'),location.href);
  if(!['audible.com','www.audible.com'].includes(u.hostname))continue;
  const match=u.pathname.match(/\/pd\/(?:[^/]+\/)?([A-Z0-9]{10})(?:\/|$)/);
  if(!match)continue;
  const title=(a.textContent||'').trim().replace(/\s+/g,' ').slice(0,1000);
  if(!title)continue;
  books.set(match[1],{asin:match[1],title,author:'',series:'',volume:0,duration:0,chapters:[]});
  if(books.size===100)break;
 }
 location.href='audible-library://snapshot?payload='+encodeURIComponent(JSON.stringify([...books.values()]));
})();
