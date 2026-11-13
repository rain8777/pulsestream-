const {invoke}=window.__TAURI__.core,{listen}=window.__TAURI__.event,$=id=>document.getElementById(id);
const P={play:"M8 5v14l11-7z",pause:"M6 5h4v14H6zM14 5h4v14h-4z",next:"M6 6l8.5 6L6 18zM16 6h2v12h-2z",prev:"M18 6l-8.5 6 8.5 6zM6 6h2v12H6z",
shuf:"M10.6 9.2L5.4 4 4 5.4l5.2 5.2zM14.5 4l2 2L4 18.6 5.4 20 18 7.5l2 2V4zM14.8 13.4l-1.4 1.4 3.1 3.1-2 2H20v-5.5l-2 2z",
rep:"M7 7h10v3l4-4-4-4v3H5v6h2zM17 17H7v-3l-4 4 4 4v-3h12v-6h-2z",
heart:"M12 21l-1.5-1.3C5.4 15.4 2 12.3 2 8.5 2 5.4 4.4 3 7.5 3c1.7 0 3.4.8 4.5 2.1C13.1 3.8 14.8 3 16.5 3 19.6 3 22 5.4 22 8.5c0 3.8-3.4 6.9-8.5 11.2z",
add:"M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6z"};
const ic=d=>`<svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true"><path d="${d}"/></svg>`;
for(const k of["shuf","prev","next"])$(k).innerHTML=ic(P[k]);
const setPP=p=>$("pp").innerHTML=ic(p?P.pause:P.play);setPP(false);
const setRep=()=>{$("rep").innerHTML=ic(P.rep)+(rep===2?"<sup>1</sup>":"");$("rep").classList.toggle("on",rep>0);$("rep").title="Repeat: "+["off","all","one"][rep]};
const a=new Audio();a.preload="none";a.volume=.8;
let view="search",searchRes=[],res=[],q=[],i=-1,shuf=false,rep=0,timer,req=0,drag=false,msg="Search for a song to begin.",saved=null;
let pls=[],target=null,curPl=null,dragT=null,retried=false,sleepMin=0,sleepT,lyrFor="",autoOn=false;
const liked=new Set(),key=t=>t.provider+":"+t.id;setRep();
const esc=s=>String(s).replace(/[&<>"']/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]));
const once=f=>{let t=0;return()=>{const n=Date.now();if(n-t>250){t=n;f()}}};
const parse=l=>l.map(j=>{try{return JSON.parse(j)}catch{return null}}).filter(Boolean);
let tt;const toast=t=>{const e=$("toast");e.textContent=t;e.classList.add("show");clearTimeout(tt);tt=setTimeout(()=>e.classList.remove("show"),1800)};
invoke("likes").then(l=>{parse(l).forEach(t=>liked.add(key(t)));render()}).catch(()=>{});

function chips(){$("chips").innerHTML=pls.map(([id,n])=>`<button class="chip${id===target?" on":""}" data-chip="${id}" title="Select as target, or drop a track here">${esc(n)}</button>`).join("")}
async function loadPls(){try{pls=await invoke("playlists");if(!pls.some(p=>p[0]===target))target=pls.length?pls[0][0]:null}catch{pls=[]}chips()}
function render(){const L=$("list");
 if(view==="settings"){L.innerHTML=`<div class="set"><label><input type="checkbox" id="auto" ${autoOn?"checked":""}> Start with Windows (minimized to tray)</label><label>Sleep timer <select id="sleep">${[0,15,30,45,60,90].map(m=>`<option value="${m}"${m===sleepMin?" selected":""}>${m?m+" min":"Off"}</option>`).join("")}</select></label></div>`;return}
 if(view==="playlists"){L.innerHTML=`<div class="pl-new"><input id="plname" placeholder="New playlist name" aria-label="New playlist name" maxlength="60"><button id="plcreate">Create</button></div>`+(pls.map(([id,n])=>`<div class="row" role="listitem" tabindex="0" data-pl="${id}"><div class="t">${esc(n)}</div><button class="like" data-pldel="${id}" aria-label="Delete playlist ${esc(n)}">×</button></div>`).join("")||`<p class="empty">No playlists yet. Create one above.</p>`);return}
 const cur=q[i]?key(q[i]):"";
 L.innerHTML=(view==="pl"?`<div class="pl-new"><button id="plback">← Playlists</button></div>`:"")+(res.length?res.map((t,n)=>`<div class="row${key(t)===cur?" cur":""}" role="listitem" tabindex="0" draggable="true" data-n="${n}"><img loading="lazy" src="${esc(t.art||"")}" alt=""><div class="t">${esc(t.title)}<br><small>${esc(t.artist)} · ${esc(t.provider)}</small></div><button class="like" data-add="${n}" aria-label="Add to selected playlist" title="Add to selected playlist">${ic(P.add)}</button><button class="like${liked.has(key(t))?" on":""}" data-like="${n}" aria-label="Like" aria-pressed="${liked.has(key(t))}">${ic(P.heart)}</button></div>`).join(""):`<p class="empty">${esc(msg)}</p>`)}
async function show(v){view=v;curPl=null;for(const b of document.querySelectorAll("#nav button"))b.classList.toggle("on",b.dataset.view===(v==="pl"?"playlists":v));
 try{
  if(v==="search"){res=searchRes;msg=$("q").value.trim()?"No playable results.":"Search for a song to begin."}
  else if(v==="recent"){const seen=new Set();res=parse(await invoke("recent")).filter(t=>!seen.has(key(t))&&seen.add(key(t)));msg="Nothing played yet."}
  else if(v==="liked"){res=parse(await invoke("likes"));msg="No liked songs yet."}
  else if(v==="playlists")await loadPls();
  else if(v==="settings")autoOn=await invoke("autostart_get").catch(()=>false);
 }catch{res=[];msg="Couldn't load this list."}
 render()}
async function openPl(id){view="pl";curPl=id;try{res=parse(await invoke("playlist_tracks",{pid:id}))}catch{res=[]}msg="This playlist is empty.";render()}
async function search(){const v=$("q").value.trim(),my=++req;if(view!=="search")await show("search");
 if(!v){searchRes=res=[];msg="Search for a song to begin.";return render()}
 try{const r=await invoke("search",{q:v});if(my!==req)return;searchRes=res=r.filter(t=>t.stream_url);msg="No playable results."}
 catch{if(my!==req)return;searchRes=res=[];msg="Search failed. Check your connection."}render()}
function play(n){if(n<0||n>=q.length)return;i=n;retried=false;const t=q[n];a.src=t.stream_url;a.play().catch(()=>{});
 $("now").textContent=`${t.title} — ${t.artist}`;$("seek").value=0;invoke("played",{track:t}).catch(()=>{});render();loadLyrics();
 if("mediaSession"in navigator)navigator.mediaSession.metadata=new MediaMetadata({title:t.title,artist:t.artist,album:t.album,artwork:t.art?[{src:t.art}]:[]})}
function next(){if(!q.length)return;
 if(shuf&&q.length>1){let n;do n=Math.floor(Math.random()*q.length);while(n===i);return play(n)}
 if(i+1<q.length)play(i+1);else if(rep>0)play(0)}
function prev(){if(a.currentTime>3||i<=0)a.currentTime=0;else play(i-1)}
function pp(){if(i<0)return;a.paused?a.play().catch(()=>{}):a.pause()}
const nextK=once(next),prevK=once(prev),ppK=once(pp);
a.onplay=()=>setPP(true);a.onpause=()=>setPP(false);
a.onended=()=>{if(rep===2){a.currentTime=0;a.play().catch(()=>{})}else next()};
// Saved tracks keep old preview URLs that expire: on error, look the track up again once, else skip.
a.onerror=async()=>{const t=q[i];if(!t)return;
 if(!retried){retried=true;try{const r=await invoke("search",{q:`${t.artist} ${t.title}`});const f=r.find(x=>key(x)===key(t)&&x.stream_url&&x.stream_url!==t.stream_url);if(f){t.stream_url=f.stream_url;a.src=f.stream_url;a.play().catch(()=>{});return}}catch{}}
 $("now").textContent="Couldn't play this track";if(i+1<q.length)setTimeout(next,800)};
a.ontimeupdate=()=>{if(!drag&&a.duration)$("seek").value=a.currentTime/a.duration*100};
$("seek").addEventListener("pointerdown",()=>drag=true);$("seek").addEventListener("change",()=>drag=false);window.addEventListener("pointerup",()=>drag=false);
$("seek").oninput=e=>{if(a.duration)a.currentTime=a.duration*e.target.value/100};$("vol").oninput=e=>a.volume=e.target.value;
$("q").oninput=()=>{clearTimeout(timer);timer=setTimeout(search,350)};
const rowPlay=r=>{q=res.slice();play(+r.dataset.n)};
const addTo=(pid,t)=>invoke("playlist_add",{pid,track:t}).then(()=>toast("Added to playlist")).catch(()=>toast("Couldn't add"));
$("list").onclick=async e=>{const g=s=>e.target.closest(s);let b;
 if(b=g("[data-like]")){const t=res[+b.dataset.like];invoke("toggle_like",{track:t}).then(on=>{on?liked.add(key(t)):liked.delete(key(t));b.classList.toggle("on",on);b.setAttribute("aria-pressed",on)}).catch(()=>{});return}
 if(b=g("[data-add]")){if(target==null)return toast("Create a playlist first");return addTo(target,res[+b.dataset.add])}
 if(b=g("[data-pldel]")){await invoke("playlist_delete",{pid:+b.dataset.pldel}).catch(()=>{});await loadPls();return render()}
 if(g("#plcreate")){const n=$("plname").value;if(!n.trim())return;try{target=await invoke("playlist_create",{name:n})}catch{return toast("Couldn't create playlist")}await loadPls();return render()}
 if(g("#plback"))return show("playlists");
 if(b=g("[data-pl]"))return openPl(+b.dataset.pl);
 if(b=g(".row[data-n]"))rowPlay(b)};
$("list").onchange=e=>{if(e.target.id==="auto"){const on=e.target.checked;invoke("autostart_set",{on}).then(()=>autoOn=on).catch(()=>{e.target.checked=!on;toast("Couldn't change auto-start")})}
 if(e.target.id==="sleep"){sleepMin=+e.target.value;clearTimeout(sleepT);if(sleepMin){sleepT=setTimeout(()=>{a.pause();toast("Sleep timer: paused")},sleepMin*60000);toast(`Sleep timer: ${sleepMin} min`)}}};
$("nav").onclick=e=>{const b=e.target.closest("[data-view]");if(b)show(b.dataset.view)};
$("chips").onclick=e=>{const b=e.target.closest("[data-chip]");if(b){target=+b.dataset.chip;chips()}};
$("list").addEventListener("dragstart",e=>{const r=e.target.closest&&e.target.closest(".row[data-n]");dragT=r?res[+r.dataset.n]:null;if(r)e.dataTransfer.setData("text/plain","track")});
$("chips").addEventListener("dragover",e=>{const c=e.target.closest("[data-chip]");if(c&&dragT){e.preventDefault();c.classList.add("drop")}});
$("chips").addEventListener("dragleave",e=>{const c=e.target.closest("[data-chip]");if(c)c.classList.remove("drop")});
$("chips").addEventListener("drop",e=>{e.preventDefault();const c=e.target.closest("[data-chip]");if(c){c.classList.remove("drop");if(dragT)addTo(+c.dataset.chip,dragT)}dragT=null});
$("pp").onclick=pp;$("next").onclick=next;$("prev").onclick=prev;
$("shuf").onclick=e=>e.currentTarget.classList.toggle("on",shuf=!shuf);$("rep").onclick=()=>{rep=(rep+1)%3;setRep()};
async function loadLyrics(){const t=q[i];if($("lyr").hidden||!t)return;const k=key(t);lyrFor=k;$("lyrtext").textContent="Loading lyrics…";
 try{const l=await invoke("lyrics",{artist:t.artist,title:t.title,duration:t.duration});if(lyrFor===k)$("lyrtext").textContent=l||"No lyrics found for this track."}
 catch{if(lyrFor===k)$("lyrtext").textContent="Couldn't load lyrics."}}
$("lyrbtn").onclick=()=>{const h=$("lyr").hidden=!$("lyr").hidden;document.body.classList.toggle("lyr",!h);$("lyrbtn").classList.toggle("on",!h);if(!h)loadLyrics()};
const root=document.documentElement,dark=()=>root.dataset.theme?root.dataset.theme==="dark":matchMedia("(prefers-color-scheme: dark)").matches;
try{const s=localStorage.getItem("theme");if(s)root.dataset.theme=s}catch{}
$("theme").onclick=()=>{const n=dark()?"light":"dark";root.dataset.theme=n;try{localStorage.setItem("theme",n)}catch{}};
$("mini").onclick=async()=>{const w=window.__TAURI__.window.getCurrentWindow(),D=window.__TAURI__.dpi;
 try{const m=!document.body.classList.contains("mini");
  if(m){saved=await w.innerSize();await w.setSize(new D.LogicalSize(420,64))}else await w.setSize(saved||new D.LogicalSize(980,640));
  document.body.classList.toggle("mini",m);await w.setAlwaysOnTop(m)}catch(e){console.error(e)}};
listen("media-key",e=>({play:ppK,next:nextK,prev:prevK}[e.payload]||(()=>{}))());
if("mediaSession"in navigator){const m=navigator.mediaSession;m.setActionHandler("play",ppK);m.setActionHandler("pause",ppK);m.setActionHandler("nexttrack",nextK);m.setActionHandler("previoustrack",prevK)}
document.addEventListener("keydown",e=>{const t=e.target.tagName;if((t==="INPUT"&&e.target.type!=="range")||t==="SELECT")return;
 if(e.code==="Space"&&t!=="BUTTON"){e.preventDefault();pp()}
 else if(e.ctrlKey&&e.code==="ArrowRight")next();else if(e.ctrlKey&&e.code==="ArrowLeft")prev();
 else if(e.key==="Enter"&&e.target.classList.contains("row")){const r=e.target;r.dataset.pl?openPl(+r.dataset.pl):rowPlay(r)}});
loadPls();render();
