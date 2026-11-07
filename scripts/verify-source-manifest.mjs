#!/usr/bin/env node
// A release must not carry a self-verification file that fails on its own source.
import {readFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {dirname,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
export function verifyManifest(text,files,read){
 const expected=new Set(files.filter(name=>name!=='MANIFEST.sha256')),seen=new Set();
 for(const line of text.trimEnd().split('\n')){
  const match=/^([a-f0-9]{64})  (.+)$/.exec(line);
  if(!match||!expected.has(match[2])||seen.has(match[2]))throw Error('Unexpected or duplicate source manifest entry');
  const [,hash,name]=match;seen.add(name);
  if(createHash('sha256').update(read(name)).digest('hex')!==hash)throw Error('Source manifest mismatch: '+name);
 }
 if(seen.size!==expected.size)throw Error('Source manifest has missing tracked files');
 return seen.size;
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
 const files=execFileSync('git',['ls-files','-z'],{cwd:root,encoding:'utf8'}).split('\0').filter(Boolean);
 try{console.log('Source manifest verified: '+verifyManifest(readFileSync(root+'/MANIFEST.sha256','utf8'),files,name=>readFileSync(root+'/'+name))+' files');}catch(error){console.error(error.message);process.exitCode=1;}
}
