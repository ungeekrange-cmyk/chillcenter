const fs=require('node:fs'),vm=require('node:vm');
const {catalogWindow}=require('../catalog-window.js');
class Element{
 constructor(tag='div',text='',cls=''){this.tagName=tag.toUpperCase();this.textContent=text||'';this.className=cls;this.children=[];this.dataset={};this.style={};this.attributes={};this.value='';this.hidden=false;this.disabled=false;this.isConnected=true;this.scrollTop=0;this.clientHeight=600;this.events={};this.elements={name:new NameInput()};const classes=new Set();this.classList={add:(...a)=>a.forEach(x=>classes.add(x)),remove:(...a)=>a.forEach(x=>classes.delete(x)),contains:x=>classes.has(x),toggle:(x,on)=>on?classes.add(x):classes.delete(x)};}
 append(...children){this.children.push(...children);children.forEach(c=>{c.parentElement=this;});}prepend(...c){this.children.unshift(...c);}replaceChildren(...c){this.children=[];this.append(...c);}setAttribute(k,v){this.attributes[k]=v;}addEventListener(k,v){this.events[k]=v;}getBoundingClientRect(){return {width:1000,top:0,x:0,y:0,height:this.clientHeight};}querySelector(){return new Element();}before(){}remove(){this.isConnected=false;}showModal(){this.open=true;}close(){this.open=false;}focus(){}reset(){this.elements.name.value='';}contains(n){return this.children.includes(n);}closest(){return null;}
 get firstChild(){return this.children[0];}
 get lastChild(){return this.children[this.children.length-1];}
}
class NameInput{constructor(){this.value='';}focus(){}}
function fixture(read){
 const nodes=new Map(),calls=[];const get=id=>{if(!nodes.has(id)){const e=new Element();e.id=id;e.elements.q=new NameInput();nodes.set(id,e);}return nodes.get(id);};
 const sandbox={$:get,node:(...a)=>new Element(...a),state:{server_id:'s',profile_id:'p'},busy:false,playerBusy:false,shellSection:'catalog',CATALOG_PAGE_SIZE:48,catalogWindow,innerHeight:800,
  document:{body:new Element(),createElement:tag=>new Element(tag),activeElement:new Element()},window:{addEventListener(){}},scrollTo(){},stopPlayerForNavigation:async()=>true,message(){},
  invoke:async c=>{calls.push(c);return read(c);},run:async c=>{calls.push(c);return read(c);},setTimeout,clearTimeout,requestAnimationFrame:fn=>{fn();return 1;},IntersectionObserver:class{constructor(callback){sandbox.artworkObserverCallback=callback;}observe(){}unobserve(){}disconnect(){}}};
 sandbox.shellShow=section=>{sandbox.shellSection=section;for(const [id,name]of [['collections-panel','collections'],['collection-panel','collection'],['picker-panel','picker']])get(id).hidden=name!==section;};
 vm.createContext(sandbox);const app=fs.readFileSync(__dirname+'/../app.js','utf8');vm.runInContext(app.slice(app.indexOf('function catalogCommand'),app.indexOf('\nfunction catalogFilters')),sandbox);vm.runInContext(fs.readFileSync(__dirname+'/../catalog.js','utf8'),sandbox);
 return {get,calls,sandbox,emitArtwork:entries=>sandbox.artworkObserverCallback(entries),execute:code=>vm.runInContext(code,sandbox),json:code=>JSON.parse(vm.runInContext('JSON.stringify('+code+')',sandbox))};
}
module.exports={fixture,Element};
