// Start library automation at DOM readiness, without waiting for ads/images.
(()=>{
 if(location.protocol!=='https:'||!['www.audible.com','audible.com'].includes(location.hostname)||!location.pathname.startsWith('/library'))return;
 const ready=()=>{location.href='audible-launch://result?code=ready';};
 if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',ready,{once:true});else ready();
})();
