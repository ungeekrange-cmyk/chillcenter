import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {verifyManifest} from './verify-source-manifest.mjs';
const files=['a','b','MANIFEST.sha256'],read=name=>Buffer.from(name),line=name=>createHash('sha256').update(read(name)).digest('hex')+'  '+name+'\n';
test('release source manifest verifies every tracked byte and excludes itself',()=>assert.equal(verifyManifest(line('a')+line('b'),files,read),2));
test('changed bytes, missing entries and duplicate or untracked paths block source packaging',()=>{
 assert.throws(()=>verifyManifest(line('a')+line('b'),files,()=>Buffer.from('changed')));
 for(const text of [line('a'),line('a')+line('a'),line('../outside')+line('b')])assert.throws(()=>verifyManifest(text,files,read));
});
