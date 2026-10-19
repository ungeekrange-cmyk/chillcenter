#!/usr/bin/env node
// Prepare immutable signed release files locally. Publication is separate.
import {readFileSync,writeFileSync,statSync,lstatSync,existsSync,mkdirSync,copyFileSync,mkdtempSync,rmSync} from "node:fs";
import {resolve,dirname,basename,join} from "node:path";
import {tmpdir} from "node:os";
import {fileURLToPath} from "node:url";
import {createHash} from "node:crypto";
import {execFileSync} from "node:child_process";
const project=resolve(dirname(fileURLToPath(import.meta.url)),"..");
export function validateAssetInput(component,target,bytes){
 if(!bytes.length||bytes.length>256*1024*1024)throw Error("Release asset size is outside bounds");
 if(component==="mac"){
  if(target!=="darwin-aarch64"||bytes[0]!==0x1f||bytes[1]!==0x8b)throw Error("Mac updater requires a gzip app archive");
 }else if(component==="server"){
  if(target.startsWith("unit-")||target.startsWith("installer-")){
   const units=new Set(["unit-chillcenter-service","unit-chillcenter-update-service","unit-chillcenter-update-timer"]),installers=new Set(["installer-linux","installer-web","installer-themes"]);
   if((!units.has(target)&&!installers.has(target))||bytes.length>65536||bytes.includes(0))throw Error("Invalid installer/unit asset");
   const text=bytes.toString("utf8");
   if((units.has(target)&&!text.includes("[Unit]"))||(installers.has(target)&&!/^#!\/(?:bin\/bash(?: -p)?|usr\/bin\/env bash)\n/.test(text)))throw Error("Invalid installer/unit format");
  }else if(!/^(?:linux-(?:aarch64|armv7|x86_64)|updater-(?:aarch64-unknown-linux-gnu|armv7-unknown-linux-gnueabihf|x86_64-unknown-linux-gnu))$/.test(target)||!bytes.subarray(0,4).equals(Buffer.from([0x7f,69,76,70])))throw Error("Linux component requires a reviewed ELF binary");
 }else if((component==="web"&&target!=="web")||(component==="themes"&&target!=="universal")||bytes[0]!==0x50||bytes[1]!==0x4b)throw Error("Web/theme component requires a ZIP for its logical target");
 // Never sign an accidentally selected private key or credential text file.
 if(bytes.length<=65536&&/-----BEGIN (?:[A-Z]+ )*PRIVATE KEY-----|untrusted comment: minisign encrypted secret key/i.test(bytes.toString("utf8")))throw Error("Private key material is not a release asset");
}
export function manifestFor({component,version,source,assets,keyId,sequence,previousSequence=0,now=new Date()}){
 if(!["mac","server","web","themes"].includes(component)||!/^\d+\.\d+\.\d+$/.test(version)||!/^[a-f0-9]{40}$/.test(source)||!assets.length||assets.length>32)throw Error("Invalid release identity");
 if(!Number.isSafeInteger(sequence)||sequence<=previousSequence||sequence<1||!Number.isSafeInteger(previousSequence)||previousSequence<0)throw Error("Release sequence must strictly increase");
 const api={min:"2026.10.14",max:"2026.10.14"};
 return {schema_version:1,component,version,release_sequence:sequence,release_tag:`${component}-v${version}`,source_commit:source,issued_at:now.toISOString(),expires_at:new Date(now.getTime()+365*86400000).toISOString(),key_id:keyId,compatibility:{os_arch:assets.map(a=>a.target),api,...(component==="web"?{required_server:{min:"0.7.0",max:"0.999.999"}}:{}),...(component==="server"?{database:{current_schema:3,target_schema:3,read_min:3,read_max:3,write_min:3,write_max:3,rollback_min:3,rollback_max:3}}:{}),...(component==="themes"?{renderer:{theme_format:1,renderer_min:1,renderer_max:1}}:{})},assets};
}
// Explicit source-controlled reservations, not wall clocks. Check every stable
// published signed receipt. Network errors never imply "first release".
function previousPublishedSequence(component,version,publicKey){
 const repo="ungeekrange-cmyk/chillcenter",temporary=mkdtempSync(join(tmpdir(),"chill-release-ledger-"));
 let highest=0,count=0;
 try{
  for(let page=1;page<=4;page++){
   const rows=JSON.parse(execFileSync("gh",["api",`repos/${repo}/releases?per_page=100&page=${page}`],{encoding:"utf8",maxBuffer:8*1024*1024}));
   if(!Array.isArray(rows))throw Error("Invalid release listing");
   for(const release of rows){
    if(release.tag_name===`${component}-v${version}`)throw Error("Immutable component release already exists");
    if(release.draft||release.prerelease||!new RegExp(`^${component}-v\\d+\\.\\d+\\.\\d+$`).test(release.tag_name))continue;
    if(++count>100)throw Error("Component history exceeds bounded release audit");
    const name=`${component}-manifest.json`,assets=release.assets||[];
    for(const [file,cap]of [[name,65536],[name+".minisig",16384]]){const item=assets.find(a=>a.name===file);if(!item||!Number.isSafeInteger(item.size)||item.size<1||item.size>cap)throw Error("Published component receipt is missing or oversized");}
    const destination=join(temporary,String(count));mkdirSync(destination,{mode:0o700});
    execFileSync("gh",["release","download",release.tag_name,"--repo",repo,"--pattern",name+"*","--dir",destination],{stdio:["ignore","pipe","pipe"],maxBuffer:1024*1024});
    const manifest=join(destination,name);
    execFileSync("minisign",["-V","-p",publicKey,"-m",manifest,"-x",manifest+".minisig"],{stdio:["ignore","pipe","pipe"]});
    const receipt=JSON.parse(readFileSync(manifest,"utf8"));
    if(receipt.component!==component||receipt.release_tag!==release.tag_name||!Number.isSafeInteger(receipt.release_sequence)||receipt.release_sequence<1)throw Error("Published release identity is invalid");
    highest=Math.max(highest,receipt.release_sequence);
   }
   if(rows.length<100)return highest;
  }
  throw Error("Release listing exceeds bounded pagination");
 }finally{rmSync(temporary,{recursive:true,force:true});}
}
function sha(path){return createHash("sha256").update(readFileSync(path)).digest("hex");}
function main(){
 const args=process.argv.slice(2);if(args.includes("--help")){console.log("Usage: component-release.mjs --component mac|server|web|themes --input /archive --target <triple|any> --output /new-directory --sign-key /private/release.key [--extra target=/file ...]");return;}
 const options={},extra=[];for(let i=0;i<args.length;i+=2){if(args[i]==="--extra")extra.push(args[i+1]);else if(["--component","--input","--target","--output","--sign-key"].includes(args[i]))options[args[i].slice(2)]=args[i+1];else throw Error("Unknown option");}
 const {component,input,target,output}=options,key=options["sign-key"];
 if(!input||!output||!key||![input,output,key].every(p=>resolve(p)===p)||existsSync(output))throw Error("Use absolute input/key and a new output directory");
 if(resolve(key).startsWith(project+"/"))throw Error("The private signing key must stay outside Git");
 const keyMeta=lstatSync(key);if(!keyMeta.isFile()||keyMeta.isSymbolicLink()||(keyMeta.mode&0o077)!==0)throw Error("Signing key must be a private regular file (0600)");
 if(execFileSync("git",["status","--porcelain","--untracked-files=normal"],{cwd:project,encoding:"utf8"}).trim())throw Error("Commit the exact source before packaging");
 const source=execFileSync("git",["rev-parse","HEAD"],{cwd:project,encoding:"utf8"}).trim(),versions=JSON.parse(readFileSync(join(project,"versions.json"),"utf8")),version=versions[component];
 const publicKey=join(project,"updates/release.pub"),pub=readFileSync(publicKey,"utf8"),keyId=/public key ([A-F0-9]+)/.exec(pub)?.[1];if(!keyId)throw Error("Missing pinned public key identity");
 const reservation=JSON.parse(readFileSync(join(project,"updates/sequences.json"),"utf8"))[component];
 if(reservation?.version!==version||!Number.isSafeInteger(reservation?.sequence)||reservation.sequence<1)throw Error("Reserve this component version and sequence in Git first");
 const previousSequence=previousPublishedSequence(component,version,publicKey);
 if(reservation.sequence<=previousSequence)throw Error("Sequence is not newer than published signed receipts");
 const inputs=[[target,input],...extra.map(value=>{const index=value.indexOf("=");if(index<1)throw Error("Extra format is target=/absolute/file");return [value.slice(0,index),value.slice(index+1)];})];
 const targets=new Set(),names=new Set();for(const [t,path]of inputs){if(resolve(path)!==path||!/^[a-z0-9-]+$/.test(t)||targets.has(t)||names.has(basename(path))||!/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(basename(path))||!lstatSync(path).isFile()||lstatSync(path).isSymbolicLink())throw Error("Duplicate/invalid target or asset");targets.add(t);names.add(basename(path));}
 mkdirSync(output,{mode:0o700});
 const assets=inputs.map(([t,path])=>{const file=basename(path),destination=join(output,file);if(statSync(path).size>256*1024*1024)throw Error("Asset is too large");copyFileSync(path,destination,1);validateAssetInput(component,t,readFileSync(destination));const asset={target:t,url:`https://github.com/ungeekrange-cmyk/chillcenter/releases/download/${component}-v${version}/${file}`,byte_length:statSync(destination).size,sha256:sha(destination),format:component==="mac"?"tar":component==="server"?"raw":"zip"};
  if(component==="mac"||t.startsWith("installer-")){const signature=destination+".minisig";execFileSync("minisign",["-S","-s",key,"-m",destination,"-x",signature,"-t",`timestamp:${Math.floor(Date.now()/1000)}\tfile:${file}\tversion:${version}`],{stdio:["ignore","pipe","pipe"]});if(component==="mac")asset.tauri_signature=readFileSync(signature).toString("base64");}return asset;});
 const manifest=manifestFor({component,version,source,assets,keyId,sequence:reservation.sequence,previousSequence}),path=join(output,`${component}-manifest.json`);
 writeFileSync(path,JSON.stringify(manifest,null,2)+"\n",{flag:"wx"});
 execFileSync("minisign",["-S","-s",key,"-m",path,"-x",path+".minisig","-t",`component:${component}\tversion:${version}\tsource:${source}`],{stdio:["ignore","pipe","pipe"]});
 execFileSync("minisign",["-V","-p",join(project,"updates/release.pub"),"-m",path,"-x",path+".minisig"],{stdio:["ignore","pipe","pipe"]});
 copyFileSync(join(project,"updates/release.pub"),join(output,"release.pub"),1);
 console.log(JSON.stringify({component,version,source,manifest_sha256:sha(path),output}));
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){try{main();}catch(error){console.error(error.message);process.exitCode=1;}}
