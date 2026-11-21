document.querySelectorAll('pre').forEach(function(p){var b=document.createElement('button');b.className='copy';b.textContent='copy';b.onclick=function(){navigator.clipboard.writeText(p.innerText.replace(/\ncopy$/,'').trim());b.textContent='copied';setTimeout(function(){b.textContent='copy'},1400)};p.appendChild(b)});
var Q=function(s){return document.querySelector(s)};
var art=Q('main article'),toc=Q('#toc');
if(art&&toc){var hs=[].slice.call(art.querySelectorAll(':scope > h2[id], :scope > h3[id]'));
if(hs.length){var links={};hs.forEach(function(h){var a=document.createElement('a');a.href='#'+h.id;a.textContent=h.textContent;a.className=h.tagName==='H3'?'l3':'';toc.appendChild(a);links[h.id]=a});
var io=new IntersectionObserver(function(es){es.forEach(function(e){if(e.isIntersecting){for(var k in links)links[k].classList.remove('on');links[e.target.id].classList.add('on')}})},{rootMargin:'-70px 0px -70% 0px'});hs.forEach(function(h){io.observe(h)})}
else{toc.previousElementSibling.style.display='none';toc.style.display='none'}}
var sm=Q('#sm'),sq=Q('#sq'),sr=Q('#sr'),IDX=window.BENTO_INDEX||[],cur=[],sel=0;
function score(e,q){var t=e.t.toLowerCase(),p=e.p.toLowerCase(),d=(e.d||'').toLowerCase(),s=0;q.split(/\s+/).forEach(function(w){if(!w)return;if(t.indexOf(w)===0)s+=6;else if(t.indexOf(w)>-1)s+=4;else if(p.indexOf(w)>-1)s+=2;else if(d.indexOf(w)>-1)s+=1;else s-=100});return s}
function paint(){[].forEach.call(sr.children,function(a,i){a.classList.toggle('on',i===sel)})}
function render(q){q=q.toLowerCase().trim();cur=q?IDX.map(function(e){return[score(e,q),e]}).filter(function(x){return x[0]>0}).sort(function(a,b){return b[0]-a[0]}).slice(0,9).map(function(x){return x[1]}):IDX.filter(function(e){return!e.h}).slice(0,9);sel=0;sr.innerHTML=cur.length?cur.map(function(e){return'<a href="'+e.u+'"><b>'+e.t+'</b><span>'+(e.h?e.p:'page')+'</span></a>'}).join(''):'<p class="none">Nothing found.</p>';paint()}
function openS(){sm.hidden=false;sq.value='';render('');sq.focus()}function closeS(){sm.hidden=true}
Q('#sb').onclick=openS;sm.onclick=function(e){if(e.target===sm)closeS()};sq.oninput=function(){render(sq.value)};
document.addEventListener('keydown',function(e){var typing=/INPUT|TEXTAREA/.test(document.activeElement.tagName);
if((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='k'){e.preventDefault();openS()}
else if(e.key==='/'&&!typing){e.preventDefault();openS()}
else if(!sm.hidden){if(e.key==='Escape')closeS();else if(e.key==='ArrowDown'){e.preventDefault();sel=Math.min(sel+1,cur.length-1);paint()}else if(e.key==='ArrowUp'){e.preventDefault();sel=Math.max(sel-1,0);paint()}else if(e.key==='Enter'&&cur[sel]){location.href=cur[sel].u}}});
