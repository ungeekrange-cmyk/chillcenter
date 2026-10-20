#!/usr/bin/env node
// Deliberately dependency-free: it is also run on the Pi build host.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = process.cwd();
const specPath = join(root, 'docs/api/openapi-2026-10-14.json');
const spec = JSON.parse(readFileSync(specPath, 'utf8'));
const fail = (message) => { console.error(`API contract: ${message}`); process.exitCode = 1; };
if (spec.openapi !== '3.1.0') fail('OpenAPI 3.1.0 attendu');
if (spec.info?.version !== '2026-10-14') fail('version datée attendue');
if (spec.servers?.length !== 1 || !spec.servers[0].url.endsWith('/api/2026-10-14')) fail('une seule base canonique attendue');
if (!spec.paths || Object.keys(spec.paths).length < 35) fail('couverture de routes insuffisante');
const operationIds = new Set();
for (const [path, item] of Object.entries(spec.paths ?? {})) {
  if (!path.startsWith('/')) fail(`chemin invalide: ${path}`);
  for (const [method, operation] of Object.entries(item)) {
    if (method === 'parameters') continue;
    if (!['get', 'post', 'put', 'delete'].includes(method)) fail(`${path}: méthode invalide ${method}`);
    if (!operation.operationId) fail(`${path} ${method}: operationId manquant`);
    if (operationIds.has(operation.operationId)) fail(`${path} ${method}: operationId dupliqué ${operation.operationId}`);
    operationIds.add(operation.operationId);
    if (!operation.responses || Object.keys(operation.responses).length === 0) fail(`${path} ${method}: réponses manquantes`);
  }
}
const schemas = spec.components?.schemas ?? {};
for (const name of ['Error', 'Media', 'Profile', 'Preferences', 'PlaybackControl', 'Progress', 'TmdbConfig', 'ImportMapping']) {
  if (!schemas[name]) fail(`schéma requis absent: ${name}`);
}
if (schemas.Error?.required?.join(',') !== 'code,message,request_id') fail('schéma d’erreur instable');
if (schemas.TmdbConfig?.properties?.credential?.writeOnly !== true) fail('secret TMDB doit être writeOnly');
if (schemas.Media?.properties?.path?.description?.includes('jamais URL NAS') !== true) fail('le chemin média doit rester non exposant');
const serverSource = readFileSync(join(root, 'crates/chill-server/src/api.rs'), 'utf8');
const versionSource = readFileSync(join(root, 'crates/chill-server/src/api_version.rs'), 'utf8');
if (!serverSource.includes('.nest(crate::api_version::PREFIX') || serverSource.includes('.nest("/api/v1"')) fail('seule la route canonique doit être servie');
if (!/VERSION\s*:\s*&str\s*=\s*"2026-10-14"/.test(versionSource)) fail('version Rust et OpenAPI divergent');
const requiredPaths = [
  '/server', '/profiles', '/admin/library', '/admin/scan', '/profiles/{profile}/catalog',
  '/profiles/{profile}/media/{media}', '/profiles/{profile}/sagas', '/profiles/{profile}/playlists',
  '/profiles/{profile}/playback/acquire', '/media/{media}/stream', '/admin/tmdb', '/admin/tmdb/jobs',
  '/profiles/{profile}/ai/context', '/admin/exports', '/admin/imports', '/openapi.json'
];
for (const path of requiredPaths) if (!spec.paths[path]) fail(`route publique manquante: ${path}`);
const routerFiles = [
  'crates/chill-server/src/api.rs',
  'crates/chill-server/src/catalog_api.rs',
  'crates/chill-server/src/identity_api.rs',
  'crates/chill-server/src/playback.rs',
  'crates/chill-server/src/tmdb_api.rs',
  'crates/chill-server/src/ai_api.rs',
  'crates/chill-server/src/archive_api.rs',
  'crates/chill-server/src/export_jobs.rs'
  , 'crates/chill-server/src/sync_api.rs', 'crates/chill-server/src/updates_api.rs'
];
const routerRoutes = new Map();
for (const file of routerFiles) {
  const source = readFileSync(join(root, file), 'utf8');
  const matches = [...source.matchAll(/\.route\(\s*"([^"\\]+)"/g)];
  for (const [index, match] of matches.entries()) {
    // The Axum combinator is deliberately adjacent to its literal.  Keep the
    // inspection local so handler functions after the final route cannot be
    // mistaken for a method on that route.
    const end = Math.min(matches[index + 1]?.index ?? source.length, match.index + 384);
    const methods = new Set([...source.slice(match.index, end).matchAll(/\b(get|post|put|delete)\s*\(/g)].map((entry) => entry[1]));
    routerRoutes.set(match[1], methods);
  }
}
for (const [route, methods] of routerRoutes) {
  if (!spec.paths[route]) {
    fail(`route Rust absente du contrat: ${route}`);
    continue;
  }
  for (const method of methods) if (!spec.paths[route][method]) fail(`${route}: méthode Rust absente du contrat: ${method}`);
}
for (const path of Object.keys(spec.paths)) {
  if (path === '/openapi.json' || routerRoutes.has(path)) continue;
  fail(`route sans source Rust: ${path}`);
}
const refPattern = /^#\/components\/(schemas|parameters|responses|requestBodies)\/([^/]+)$/;
const resolveSchema = (schema, seen = new Set()) => {
  if (!schema?.$ref) return schema;
  if (seen.has(schema.$ref)) return schema;
  seen.add(schema.$ref);
  const match = schema.$ref.match(/^#\/components\/schemas\/([^/]+)$/);
  return match ? resolveSchema(schemas[match[1]], seen) : schema;
};
const checkRefs = (value, location = '$') => {
  if (Array.isArray(value)) return value.forEach((entry, index) => checkRefs(entry, `${location}[${index}]`));
  if (!value || typeof value !== 'object') return;
  if (typeof value.$ref === 'string') {
    const match = value.$ref.match(refPattern);
    if (!match || !spec.components?.[match[1]]?.[match[2]]) fail(`référence inexistante ${value.$ref} (${location})`);
  }
  for (const [key, nested] of Object.entries(value)) if (key !== '$ref') checkRefs(nested, `${location}.${key}`);
};
checkRefs(spec);
const responseChecked=new Set();
function checkResponseGraph(schema,location){
 if(!schema||typeof schema!=='object')return;
 if(schema.$ref?.startsWith('#/components/schemas/')){const name=schema.$ref.split('/').at(-1);if(!responseChecked.has(name)){responseChecked.add(name);checkResponseGraph(schemas[name],location+'/'+name);}}
 if((schema.type==='object'||schema.properties)&&schema.additionalProperties===false)fail(`réponse imbriquée non extensible: ${location}`);
 for(const [key,value]of Object.entries(schema))if(key!=='$ref')checkResponseGraph(value,location+'.'+key);
}
for(const [path,item]of Object.entries(spec.paths))for(const [method,operation]of Object.entries(item))for(const response of Object.values(operation.responses||{})){
 const resolved=response.$ref?.startsWith('#/components/responses/')?spec.components.responses[response.$ref.split('/').at(-1)]:response;
 checkResponseGraph(resolved.content,path+' '+method);
}
const mutationWithoutBody = new Set([
  'POST /admin/scan', 'POST /admin/exports', 'POST /admin/tmdb/jobs/{id}/cancel',
  'POST /admin/tmdb/jobs/{id}/retry', 'DELETE /admin/tmdb', 'DELETE /admin/exports/{id}/cancel',
  'DELETE /admin/imports/{id}', 'DELETE /profiles/{profile}/playlists/{playlist}'
]);
const emptyObject = (schema) => schema?.type === 'object' && Object.keys(schema.properties ?? {}).length === 0;
const impreciseObject = (schema) => schema?.type === 'object'
  && Object.keys(schema.properties ?? {}).length === 0
  && (schema.additionalProperties === undefined || schema.additionalProperties === true);
const findImpreciseObject = (value, seen = new Set()) => {
  const resolved = resolveSchema(value);
  if (!resolved || typeof resolved !== 'object' || seen.has(resolved)) return false;
  seen.add(resolved);
  if (impreciseObject(resolved)) return true;
  return Object.values(resolved).some((nested) => findImpreciseObject(nested, seen));
};
for (const [path, item] of Object.entries(spec.paths)) {
  for (const [method, operation] of Object.entries(item)) {
    if (!['get', 'post', 'put', 'delete'].includes(method) || path === '/openapi.json') continue;
    const success = Object.entries(operation.responses ?? {}).find(([status]) => /^2\d\d$/.test(status));
    if (!success) {
      fail(`${path} ${method}: réponse de succès absente`);
      continue;
    }
    const responseSchema = resolveSchema(success[1].content?.['application/json']?.schema);
    if (success[0] !== '204' && !success[1].content) fail(`${path} ${method}: contenu de réponse typé absent`);
    if (emptyObject(responseSchema)) fail(`${path} ${method}: réponse JSON object vide interdite`);
    if (path !== '/openapi.json' && findImpreciseObject(responseSchema)) fail(`${path} ${method}: réponse JSON imprécise interdite`);
    const mutation = `${method.toUpperCase()} ${path}`;
    if (['post', 'put'].includes(method) && !mutationWithoutBody.has(mutation) && !operation.requestBody) fail(`${mutation}: corps requis absent`);
    const requestSchema = resolveSchema(operation.requestBody?.content?.['application/json']?.schema);
    if (emptyObject(requestSchema)) fail(`${mutation}: corps JSON object vide interdit`);
    if (findImpreciseObject(requestSchema)) fail(`${mutation}: corps JSON imprécis interdit`);
    if (requestSchema?.type === 'object' && requestSchema.additionalProperties !== false) fail(`${mutation}: corps JSON doit refuser les champs inconnus`);
  }
}
const operation = (path, method) => spec.paths[path]?.[method];
const params = (path, method) => [
  ...(spec.paths[path]?.parameters ?? []), ...(operation(path, method)?.parameters ?? [])
];
const hasParam = (path, method, name, where = 'query') =>
  params(path, method).some((parameter) => parameter.in === where && parameter.name === name);
const schema = (name) => schemas[name] ?? {};
const same = (actual, expected) => JSON.stringify(actual) === JSON.stringify(expected);
if (operation('/profiles', 'get')?.responses?.[200]?.content?.['application/json']?.schema?.type !== 'array') fail('GET /profiles doit répondre un tableau');
if (!hasParam('/profiles/{profile}', 'delete', 'x-chill-confirm-delete', 'header')) fail('confirmation de suppression de profil manquante');
for (const path of ['/admin/unavailable', '/admin/reassociations']) for (const name of ['offset', 'limit']) if (!hasParam(path, 'get', name)) fail(`${path}: query ${name} manquante`);
if (!same(schema('Media').properties?.category?.enum, ['film', 'serie', 'spectacle', 'documentaire'])) fail('catégories Media divergentes');
if (schema('CatalogPage').properties?.series_counters?.additionalProperties?.properties?.series_key?.type !== 'string') fail('series_counters.series_key manquant');
if (schema('SeriesPutResponse').properties?.key?.type !== 'string') fail('SeriesPutResponse.key manquant');
if (!same(schema('RevisionPatch').properties?.category?.enum, ['film', 'serie', 'spectacle', 'documentaire'])) fail('catégories RevisionPatch divergentes');
const aiFields = ['id', 'title', 'category', 'year', 'series', 'season', 'episode', 'genres', 'saga', 'watched', 'rating', 'revision'];
if (!same(schema('AiContext').properties?.media?.items?.required, aiFields)) fail('AiMedia diverge du DTO serveur');
if (schema('AiContext').properties?.media?.items?.properties?.available) fail('AiMedia ne doit pas exposer available');
if (!same(schema('RatingRequest').required, ['rating']) || schema('RatingRequest').properties?.scope?.default !== 'video') fail('RatingRequest scope doit être optionnel, défaut video');
if (!same(schema('AcquireRequest').required, ['media_id', 'device_id'])) fail('AcquireRequest context doit être optionnel');
if (schema('SharedSeriesEdit').properties?.patch?.properties?.year?.type !== 'integer' || schema('SharedSeriesEdit').properties?.patch?.properties?.overview?.type !== 'string') fail('SeriesPatch year/overview doivent être non-null');
if (schema('PreferencesInput').properties?.subtitle_language?.minLength !== undefined) fail('subtitle_language vide doit être accepté');
for (const name of ['Lease', 'PlaybackControl', 'Progress']) if (schema(name).properties?.media_revision?.format !== 'uint64') fail(`${name}.media_revision doit être uint64`);
const tmdb = schema('TmdbConfig');
if (!same(tmdb.required, ['credential_type', 'credential', 'auto_enrich']) || tmdb.properties?.credential?.writeOnly !== true || tmdb.additionalProperties !== false) fail('TmdbConfig strict ou requis incorrect');
if (!tmdb.allOf?.some((rule) => rule.if?.properties?.auto_enrich?.const === true && rule.then?.required?.includes('profile_id'))) fail('TmdbConfig profile_id conditionnel manquant');
for (const credentialType of ['api_key', 'bearer']) if (!tmdb.allOf?.some((rule) => rule.if?.properties?.credential_type?.const === credentialType && rule.then?.properties?.credential?.pattern)) fail(`TmdbConfig validation ${credentialType} manquante`);
if (!schema('Saga').required?.includes('poster')) fail('Saga.poster doit être documenté');
const syncPost = operation('/profiles/{profile}/catalog/snapshot', 'post');
const syncPage = operation('/profiles/{profile}/catalog/snapshot/{snapshot}', 'get');
if (!syncPost || !syncPage) fail('routes de synchronisation locale manquantes');
const changesPost = operation('/profiles/{profile}/catalog/changes', 'post');
const changesPage = operation('/profiles/{profile}/catalog/changes/{snapshot}', 'get');
if (!changesPost || !changesPage) fail('routes de delta de synchronisation manquantes');
if (schema('CatalogChangesRequest').additionalProperties !== false) fail('CatalogChangesRequest doit refuser les champs inconnus');
for (const name of ['CatalogChangesPage']) {
  const changes = schema(name);
  if (changes.properties?.items?.maxItems !== 100) fail(`${name}: borne de page divergente`);
  if (changes.properties?.total?.maximum !== 10000) fail(`${name}: borne total divergente`);
}
if (!same(schema('CatalogChangesResponse').oneOf?.map((entry) => entry.$ref), ['#/components/schemas/CatalogChangesUnchanged', '#/components/schemas/CatalogChangesPage'])) fail('CatalogChangesResponse doit être l’union inchangé/delta');
if (!hasParam('/profiles/{profile}/catalog/changes/{snapshot}', 'get', 'offset')) fail('pagination delta incomplète');
if (schema('SyncSnapshotRequest').additionalProperties !== false) fail('SyncSnapshotRequest doit refuser les champs inconnus');
if (!same(schema('SyncSnapshotResponse').oneOf?.map((entry) => entry.$ref), [
  '#/components/schemas/SyncSnapshotUnchanged', '#/components/schemas/SyncSnapshotFirstPage'
])) fail('SyncSnapshotResponse doit être l’union inchangé/première page');
for (const name of ['SyncSnapshotFirstPage', 'SyncSnapshotPage']) {
  const sync = schema(name);
  if (sync.properties?.total?.maximum !== 10000 || sync.properties?.items?.maxItems !== 100) fail(`${name}: bornes total/page divergentes`);
  if (sync.additionalProperties === false) fail(`${name}: réponse doit rester extensible`);
}
if (!hasParam('/profiles/{profile}/catalog/snapshot/{snapshot}', 'get', 'offset')
  || schema('SyncSnapshotPage').properties?.next_offset?.maximum !== 10000) fail('pagination snapshot incomplète');
const playlist = schema('Playlist');
if (!playlist.properties?.poster || playlist.properties.poster.type?.join(',') !== 'string,null') fail('Playlist.poster optionnel manquant');
const playlistArtwork = operation('/profiles/{profile}/playlists/{playlist}/artwork', 'get');
const playlistPoster = operation('/profiles/{profile}/playlists/{playlist}/poster', 'put');
if (!playlistArtwork || !playlistPoster) fail('routes d’affiche de playlist manquantes');
for (const type of ['image/jpeg', 'image/png', 'image/webp']) {
  if (!playlistArtwork?.responses?.[200]?.content?.[type]) fail(`GET artwork playlist: ${type} absent`);
  if (!playlistPoster?.requestBody?.content?.[type]) fail(`PUT poster playlist: ${type} absent`);
}
if (!hasParam('/profiles/{profile}/playlists/{playlist}/poster', 'put', 'expected_revision')
  || playlistPoster?.responses?.[200]?.content?.['application/json']?.schema?.$ref !== '#/components/schemas/PlaylistEnvelope') fail('PUT poster playlist doit être CAS et retourner PlaylistEnvelope');
for (const name of ['Preferences', 'PreferencesInput']) {
  const autoplay = schema(name).properties?.autoplay_seconds;
  if (!autoplay?.enum?.includes(15) || autoplay.default !== 10) fail(`${name}.autoplay_seconds doit inclure 15 et préserver le défaut API 10`);
}
if(schema('ProfileCreateRequest').properties?.initial_preferences?.$ref!=='#/components/schemas/PreferencesInput')fail('initialisation atomique explicite des nouveaux profils requise');
for (const [path, item] of Object.entries(spec.paths)) {
  const pathNames = [...path.matchAll(/\{([^}]+)\}/g)].map((match) => match[1]);
  const declared = (item.parameters ?? []).filter((parameter) => parameter.in === 'path');
  for (const name of pathNames) {
    const matches = declared.filter((parameter) => parameter.name === name && parameter.required === true);
    if (matches.length !== 1) fail(`${path}: paramètre de chemin ${name} absent ou dupliqué`);
  }
  for (const [method, operation] of Object.entries(item)) {
    if (!['post', 'put', 'delete'].includes(method)) continue;
    if (operation.requestBody && operation.requestBody.required !== true) fail(`${path} ${method}: corps mutation doit être requis`);
  }
  for (const [method, operation] of Object.entries(item)) {
    if (!['get', 'post', 'put', 'delete'].includes(method)) continue;
    const seen = new Set();
    for (const parameter of [...(item.parameters ?? []), ...(operation.parameters ?? [])]) {
      const key = `${parameter.in}:${parameter.name}`;
      if (seen.has(key)) fail(`${path} ${method}: paramètre dupliqué ${key}`);
      seen.add(key);
    }
  }
}
if (process.exitCode) process.exit(process.exitCode);
console.log(`API contract OK: ${Object.keys(spec.paths).length} chemins, OpenAPI ${spec.info.version}`);
