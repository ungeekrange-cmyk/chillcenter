// Debug-only entry point. Manual gestures; no event simulation or automatic pass.
window.chillManualDetailQa=async()=>{
 try{
  await window.chillReady;
  const overview=Array.from({length:100},(_,i)=>"Bloc "+(i+1)+" — Résumé fictif pour tester le défilement de la fiche vidéo. Le catalogue derrière doit rester immobile.").join("\n\n");
  const videos=Array.from({length:160},(_,i)=>({id:"manual-"+i,title:i===1?"Attack the Block — TEST ISOLÉ":"Vidéo de test "+(i+1),category:"film",year:2011,path:"Films/Attack the Block - - (2011) - - Vosta.mkv",overview,genres:[],directors:[],cast:[],available:true,watched:false,poster:null}));
  // Replace UI reads only; the real Rust client remains disconnected and uses
  // CHILL_UI_QA_DATA. No household profile, media, or preference is accessed.
  invoke=async c=>{
   if(c.action==="catalog")return {items:videos.slice(c.offset,c.offset+48),has_more:c.offset+48<videos.length};
   if(c.action==="media_detail")return {media:videos.find(m=>m.id===c.media_id),revision:1};
   if(c.action==="related_media")return {items:[]};
   if(c.action==="resolve_appearance")return {ambiance:null,appearance:{mode:"off",theme:"dark"}};
   if(c.action==="refresh")return state;
   throw Error("Action désactivée : fenêtre de test isolée du défilement uniquement.");
  };
  state={...state,stage:"connected",server_id:"manual-fixture",profile_id:"manual-profile",profiles:[{id:"manual-profile",name:"TEST ISOLÉ"}],library:{revision:1,video_root:null},theme:"dark"};
  render();await loadPage();
  const reopen=node("button","Rouvrir la fiche de test","secondary");reopen.type="button";
  const open=async()=>{await window.catalogOpenMedia(videos[1].id);$("detail-play").hidden=true;};
  reopen.onclick=open;$("message").after(reopen);
  const instructions="TEST ISOLÉ — Faites défiler le résumé et essayez aussi de défiler sur le fond. Fermez la fiche avec ×, vérifiez que le catalogue défile à nouveau, puis donnez votre résultat dans le chat. Aucune vidéo ni connexion au Pi.";
  message(instructions);$("media-dialog").addEventListener("close",()=>message(instructions));
  scrollTo(0,350);await new Promise(resolve=>requestAnimationFrame(resolve));await open();
  const note=node("p","Test manuel : faites défiler cette fiche, puis le fond derrière. Aucun test ne sera déclaré réussi automatiquement.","muted");$("media-dialog").prepend(note);
  await window.__TAURI__.core.invoke("qa_result",{success:false,detail:"TRACE MANUAL_SCROLL_READY candidate=71bd7a0 synthetic-data=true disconnected=true no-video=true; awaiting human feedback"});
 }catch(error){await window.__TAURI__.core.invoke("qa_result",{success:false,detail:"manual-scroll-setup: "+error.message});}
};
