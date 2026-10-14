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
if (!spec.servers?.some(({ url }) => url.includes('/api/2026-10-14'))) fail('serveur canonique manquant');
if (!spec.paths || Object.keys(spec.paths).length < 35) fail('couverture de routes insuffisante');
for (const [path, item] of Object.entries(spec.paths ?? {})) {
  if (!path.startsWith('/')) fail(`chemin invalide: ${path}`);
  for (const [method, operation] of Object.entries(item)) {
    if (method === 'parameters') continue;
    if (!['get', 'post', 'put', 'delete'].includes(method)) fail(`${path}: méthode invalide ${method}`);
    if (!operation.operationId) fail(`${path} ${method}: operationId manquant`);
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
if (!serverSource.includes('.nest(crate::api_version::PREFIX') || !serverSource.includes('.nest("/api/v1"')) fail('routes canonique et compatibilité v1 attendues');
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
const mutationWithoutBody = new Set([
  'POST /admin/scan', 'POST /admin/exports', 'POST /admin/tmdb/jobs/{id}/cancel',
  'POST /admin/tmdb/jobs/{id}/retry', 'DELETE /admin/tmdb', 'DELETE /admin/exports/{id}/cancel',
  'DELETE /admin/imports/{id}', 'DELETE /profiles/{profile}/playlists/{playlist}'
]);
const emptyObject = (schema) => schema?.type === 'object' && Object.keys(schema.properties ?? {}).length === 0;
for (const [path, item] of Object.entries(spec.paths)) {
  for (const [method, operation] of Object.entries(item)) {
    if (!['get', 'post', 'put', 'delete'].includes(method) || path === '/openapi.json') continue;
    const success = Object.entries(operation.responses ?? {}).find(([status]) => /^2\d\d$/.test(status));
    if (!success) {
      fail(`${path} ${method}: réponse de succès absente`);
      continue;
    }
    const responseSchema = success[1].content?.['application/json']?.schema;
    if (success[0] !== '204' && !success[1].content) fail(`${path} ${method}: contenu de réponse typé absent`);
    if (emptyObject(responseSchema)) fail(`${path} ${method}: réponse JSON object vide interdite`);
    const mutation = `${method.toUpperCase()} ${path}`;
    if (['post', 'put'].includes(method) && !mutationWithoutBody.has(mutation) && !operation.requestBody) fail(`${mutation}: corps requis absent`);
    const requestSchema = operation.requestBody?.content?.['application/json']?.schema;
    if (emptyObject(requestSchema)) fail(`${mutation}: corps JSON object vide interdit`);
  }
}
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
}
if (process.exitCode) process.exit(process.exitCode);
console.log(`API contract OK: ${Object.keys(spec.paths).length} chemins, OpenAPI ${spec.info.version}`);
