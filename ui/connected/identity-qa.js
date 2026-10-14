"use strict";
window.chillIdentityQa=async()=>{const panel=[...document.querySelectorAll("#admin-panel section")].find(x=>x.querySelector("h2")?.textContent==="Identité des vidéos");if(!panel)throw Error("identity panel missing");return true;};
