// Preview only. Fixed number of CSS animated decorations; no animation timers.
(()=>{
  const sidebar=document.querySelector('.sidebar');
  const layer=document.createElement('div');layer.className='ambient-layer';layer.setAttribute('aria-hidden','true');
  for(const index of [0,1]){const walk=document.createElement('span');walk.className='rabbit-walk';const image=document.createElement('img');image.src='assets/rabbit-silhouette.svg';image.alt='';walk.append(image);layer.append(walk);}
  for(let index=0;index<10;index++){const petal=document.createElement('i');petal.className='petal';petal.style.setProperty('--left',`${(index*29)%100}%`);petal.style.setProperty('--delay',`${-index*1.7}s`);layer.append(petal);}
  sidebar.append(layer);
})();
