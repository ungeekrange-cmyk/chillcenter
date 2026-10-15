"use strict";
// Web replacement for the Tauri command bridge: translates the UiCommand set used by
// ui/connected into same-origin fetch calls against the dated Chill Center API (ADR-0014).
(function (root) {
  const Json64 = typeof module !== "undefined" ? require("./json64.js") : root.ChillJson64;
  const BASE = "/api/2026-10-14";
  const REQUIRED = ["catalog_pages", "playback_leases", "media_range"];
  const UUID = /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;
  const SAGA_ID = /^[A-Za-z0-9_-]{1,128}$/;
  const CATEGORIES = ["film", "serie", "spectacle", "documentaire"];
  const SORTS = ["title_asc", "title_desc", "year_asc", "year_desc", "added_desc", "rating_desc"];
  const MEDIA_FIELDS = ["title", "category", "year", "genres", "series", "season", "episode", "saga", "overview", "runtime_minutes", "cast", "directors", "series_overview"];
  const SERIES_FIELDS = ["name", "year", "genres", "overview", "cast", "directors"];
  const MESSAGES = {
    SCAN_ACTIVE: "Un scan est déjà en cours. Attendez sa fin avant de relancer ou de modifier le dossier.",
    ROOT_REQUIRED: "Configurez le dossier vidéo du serveur dans Administration avant de scanner.",
    CONFIG_INVALID_OR_UNWRITABLE: "Chemin invalide ou réglages non enregistrables. Vérifiez le chemin et les droits du serveur.",
    PROFILE_NAME_TAKEN: "Ce nom de profil est déjà utilisé. Choisissez un autre nom.",
    PROFILE_NOT_FOUND: "Ce profil a été supprimé. Rechargez et choisissez un profil existant.",
    MEDIA_NOT_FOUND: "Cet élément n'existe plus. Rechargez le catalogue.",
    BUSY: "Serveur occupé ou base indisponible. Réessayez dans quelques instants.",
    DATABASE_OFFLINE: "Serveur occupé ou base indisponible. Réessayez dans quelques instants.",
    REQUEST_TIMEOUT: "Délai dépassé. Rechargez pour vérifier le résultat avant de réessayer.",
    NETWORK_TIMEOUT: "Délai dépassé. Rechargez pour vérifier le résultat avant de réessayer.",
    INVALID_OPERATION: "Valeur invalide. Vérifiez les champs avant de réessayer.",
    PROFILE_CONFLICT: "Modification concurrente : rechargez les données avant de réessayer.",
    CONFIG_CONFLICT: "Modification concurrente : rechargez les données avant de réessayer.",
    CONFIG_DURABILITY_UNKNOWN: "Réglage serveur appliqué, mais durabilité disque non confirmée. Rechargez son état avant de réessayer.",
    BROWSER_ORIGIN_REFUSED: "Le serveur refuse cette origine. Ouvrez Chill Center depuis l'adresse du serveur lui-même.",
  };

  function fail(message, code, status) { return Object.assign(new Error(message), { message, code, status: status ?? null }); }
  const invalid = message => fail(message, "INVALID_OPERATION");
  const validId = id => { if (typeof id !== "string" || !UUID.test(id)) throw invalid("Identifiant invalide."); };
  const validMediaIds = ids => { if (!Array.isArray(ids) || ids.length > 3000) throw invalid("Liste trop longue."); ids.forEach(validId); };
  const segment = s => String(s).replace(/[^A-Za-z0-9\-_.~]/g, c => Array.from(new TextEncoder().encode(c), b => "%" + b.toString(16).toUpperCase().padStart(2, "0")).join(""));

  function apiError(status, body) {
    const code = body && body.code ? String(body.code) : "HTTP_ERROR";
    let message = MESSAGES[code];
    if (!message) {
      if (status === 404) message = "Profil ou élément supprimé : rechargez la page.";
      else if (status === 400 || status === 422) message = "Valeur invalide ou nom déjà utilisé. Vérifiez les champs.";
      else if (status === 409) message = "Modification concurrente : rechargez les données avant de réessayer.";
      else message = "Opération refusée ou temporairement indisponible.";
    }
    return fail(message, code, status);
  }

  // Port of crates/chill-client/src/appearance.rs.
  function resolveAppearance(appearance, year, month, day) {
    const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
    const days = month === 2 ? (leap ? 29 : 28) : [4, 6, 9, 11].includes(month) ? 30 : month >= 1 && month <= 12 ? 31 : 0;
    if (!(year >= 1 && year <= 9999) || day < 1 || day > days) throw invalid("Date locale invalide.");
    if (appearance.mode === "off") return null;
    if (appearance.mode === "manual") return appearance.manual;
    if (month === 10) return "halloween";
    if ((month === 11 && day >= 25) || (month === 12 && day <= 25)) return "christmas";
    if (month >= 3 && month <= 5) return "spring";
    if (month >= 6 && month <= 8) return "summer";
    if (month >= 9 && month <= 11) return "autumn";
    return "winter";
  }

  function catalogQuery(c) {
    const q = [["offset", String(c.offset || 0)], ["limit", "48"]];
    if (c.category) { if (!CATEGORIES.includes(c.category)) throw invalid("Catégorie invalide."); q.push(["category", c.category]); }
    if (c.q) q.push(["q", c.q]);
    if (c.genre) { if ([...c.genre].length > 200) throw invalid("Genre trop long."); q.push(["genre", c.genre]); }
    if (c.year !== null && c.year !== undefined) { if (c.year !== 0 && !(c.year >= 1850 && c.year <= 2200)) throw invalid("Année invalide."); q.push(["year", String(c.year)]); }
    if (c.watched !== null && c.watched !== undefined) q.push(["watched", String(c.watched)]);
    if (c.sort) { if (!SORTS.includes(c.sort)) throw invalid("Tri invalide."); q.push(["sort", c.sort]); }
    if (c.grouped !== null && c.grouped !== undefined) q.push(["grouped", String(c.grouped)]);
    if (c.available !== null && c.available !== undefined) q.push(["available", String(c.available)]);
    return q;
  }

  function createBridge(env) {
    const doFetch = env.fetch, storage = env.storage, player = env.player || null;
    const defaults = { theme: "system", appearance: { mode: "auto", manual: "autumn" }, playback_cache: { forward_mib: 128, back_mib: 64 }, player_presentation: { fullscreen_on_start: true }, last_profile: {} };
    let settings = (() => { try { return { ...defaults, ...JSON.parse(storage.getItem("chill.settings") || "{}") }; } catch { return { ...defaults }; } })();
    const save = () => { try { storage.setItem("chill.settings", JSON.stringify(settings)); } catch { /* private mode: keep in memory */ } };
    let conn = null;
    // Blob URLs are local to this bridge, so keep them scoped to the server/profile
    // context and drop them whenever a catalog-changing operation completes.
    const artwork = new Map();
    const catalogRevisions = new Map();
    let artworkGeneration = 0;
    let pendingSync = null, syncTicket = 0;

    const contextKey = (serverId, profileId) => `${serverId}:${profileId}`;
    const currentContext = () => conn && conn.active ? contextKey(conn.id, conn.active) : null;
    function clearArtwork() {
      artworkGeneration++;
      pendingSync = null;
      for (const url of artwork.values()) env.revoke?.(url);
      artwork.clear();
    }
    function catalogMutated() { clearArtwork(); }
    async function mutation(operation) {
      const result = await operation();
      catalogMutated();
      return result;
    }

    async function request(method, path, { body, query, headers } = {}) {
      let url = BASE + path;
      if (query && query.length) url += "?" + query.map(([k, v]) => encodeURIComponent(k) + "=" + encodeURIComponent(v)).join("&");
      const init = { method, headers: { Accept: "application/json", ...(headers || {}) }, credentials: "same-origin", cache: "no-store" };
      if (body !== undefined) { init.body = Json64.stringify(body); init.headers["Content-Type"] = "application/json"; }
      let response;
      try { response = await doFetch(url, init); }
      catch { throw fail("Connexion au serveur impossible. Vérifiez l'adresse et le certificat.", "CONNECTION_FAILED"); }
      const text = await response.text();
      let parsed = null;
      if (text) { try { parsed = Json64.parse(text); } catch { parsed = null; } }
      if (!response.ok) throw apiError(response.status, parsed);
      return parsed;
    }
    const profile = () => { if (!conn || !conn.active) throw invalid("Choisissez un profil."); return conn.active; };
    const personal = path => `/profiles/${profile()}${path}`;
    const get = (path, query) => request("GET", path, { query });

    function theme() { return ["light", "dark"].includes(settings.theme) ? settings.theme : "system"; }
    function disconnectedView(notice) {
      const view = { stage: "server", address: env.address || "", servers: [], theme: theme(), appearance: settings.appearance, playback_cache: settings.playback_cache, player_presentation: settings.player_presentation };
      if (notice) view.notice = notice;
      return view;
    }
    async function view() {
      if (!conn) return disconnectedView();
      const profiles = await get("/profiles");
      if (conn.active && !profiles.some(p => p.id === conn.active)) conn.active = null;
      conn.profiles = profiles;
      const library = await get("/admin/library");
      const preferences = conn.active ? await get(`/profiles/${conn.active}/preferences`) : null;
      return { stage: "connected", address: env.address || "", server_id: conn.id, profile_id: conn.active, profiles, library, preferences, theme: theme(), appearance: settings.appearance, playback_cache: settings.playback_cache, player_presentation: settings.player_presentation, notice: conn.notice || null };
    }
    async function connect() {
      clearArtwork();
      conn = null;
      const info = await get("/server");
      const missing = REQUIRED.filter(c => !(info.capabilities || []).includes(c));
      if (missing.length) throw fail("Ce serveur est trop ancien pour le client web (capacités manquantes : " + missing.join(", ") + ").", "SERVER_INCOMPATIBLE");
      conn = { id: info.server_id, active: null, profiles: [], notice: null, tls: info.tls };
      const last = settings.last_profile[info.server_id];
      const profiles = await get("/profiles");
      if (last && profiles.some(p => p.id === last)) conn.active = last;
      return view();
    }
    async function artworkFor(kind, id) {
      const p = profile(), captured = currentContext(), generation = artworkGeneration, key = captured + ":" + kind + ":" + id;
      if (!artwork.has(key)) {
        const resource = id === null ? kind : `${kind === "media" ? "media" : kind}/${id}`;
        const response = await doFetch(`${BASE}/profiles/${p}/${resource}/artwork`, { credentials: "same-origin" });
        if (!response.ok) throw apiError(response.status, null);
        const blob = await response.blob();
        if (blob.size > 8 * 1024 * 1024) throw invalid("Affiche non prise en charge.");
        // A response started before a mutation or context switch must never
        // recreate a stale URL after clearArtwork has invalidated it.
        if (generation !== artworkGeneration || currentContext() !== captured) throw fail("Affiche obsolète ignorée.", "ARTWORK_DISCARDED");
        artwork.set(key, env.objectUrl(blob));
      }
      return { src: artwork.get(key) };
    }
    async function syncCatalog() {
      const p = profile();
      const captured = currentContext();
      const knownRevision = catalogRevisions.get(captured);
      try {
        const response = await get(`/profiles/${p}/catalog/revision`);
        const revision = response && response.revision;
        if (typeof revision !== "string" || !revision || revision.length > 255) throw fail("Révision du catalogue invalide.", "INVALID_RESPONSE");
        // A completed request for an old profile may only report that it was
        // discarded; it must never update the active profile's revision/cache.
        if (currentContext() !== captured) return { changed: false, discarded: true };
        const changed = knownRevision !== revision;
        pendingSync = null;
        if (!changed) return { changed, revision, discarded: false };
        clearArtwork();
        pendingSync = { context: captured, revision, ticket: ++syncTicket };
        // A revision is applied only after the shared UI has committed every
        // catalog page. Failed/abandoned loads must retry the same revision.
        return { changed, revision, discarded: false, ack_ticket: pendingSync.ticket };
      } catch (error) {
        // Older web-capable servers do not have the inexpensive revision route.
        // Do not claim an unchanged catalog: request the UI's bounded normal-page
        // refresh instead, without using snapshot/delta mirror endpoints.
        if (error.code === "ROUTE_NOT_FOUND" && currentContext() === captured) {
          catalogMutated();
          return { changed: true, full_refresh: true, legacy_revision_route: true, discarded: false };
        }
        throw error;
      }
    }
    const unsupported = () => { throw fail("Cette fonction n'est pas encore disponible dans le client web.", "WEB_UNSUPPORTED"); };

    async function ui(c) {
      switch (c.action) {
        case "boot": case "connect": case "approve":
          try { return await connect(); } catch (e) { conn = null; const v = disconnectedView(e.message); return c.action === "boot" ? v : Promise.reject(e); }
        case "disconnect": conn = null; clearArtwork(); return view();
        case "refresh": return view();
        case "select_profile": {
          validId(c.id);
          if (!conn || !conn.profiles.some(p => p.id === c.id)) throw invalid("Profil absent ; rechargez la liste.");
          const previous = conn.active; conn.active = c.id; clearArtwork();
          try { const v = await view(); settings.last_profile[conn.id] = c.id; save(); return v; } catch (e) { conn.active = previous; throw e; }
        }
        case "create_profile": await request("POST", "/profiles", { body: { name: c.name } }); return view();
        case "rename_profile": validId(c.id); await request("PUT", `/profiles/${c.id}`, { body: { name: c.name, expected_revision: c.expected_revision } }); return view();
        case "delete_profile": validId(c.id); await request("DELETE", `/profiles/${c.id}`, { headers: { "x-chill-confirm-delete": c.confirm_id } }); catalogMutated(); return view();
        case "save_library": await request("PUT", "/admin/library", { body: { video_root: c.video_root ?? null, expected_revision: c.expected_revision } }); catalogMutated(); return view();
        case "save_preferences": await request("PUT", personal("/preferences"), { body: { audio_language: c.audio_language, subtitle_language: c.subtitle_language, autoplay_seconds: c.autoplay_seconds ?? null } }); return view();
        case "catalog": return get(personal("/catalog"), catalogQuery(c));
        case "sync_catalog": return syncCatalog();
        case "ack_catalog_sync": {
          if (!pendingSync || c.ticket !== pendingSync.ticket || pendingSync.context !== currentContext()) return { discarded: true };
          catalogRevisions.set(pendingSync.context, pendingSync.revision);
          pendingSync = null;
          return { discarded: false };
        }
        case "set_watched": validId(c.media_id); return mutation(() => request("PUT", personal(`/media/${c.media_id}/watched`), { body: { watched: c.watched } }));
        case "set_rating": validId(c.media_id); return mutation(() => request("PUT", personal(`/media/${c.media_id}/rating`), { body: { rating: c.rating ?? null } }));
        case "group_rating": validId(c.media_id); return mutation(() => request("PUT", personal(`/media/${c.media_id}/rating`), { body: { rating: c.rating ?? null, scope: c.scope } }));
        case "media_detail": validId(c.media_id); return get(personal(`/media/${c.media_id}`));
        case "related_media": validId(c.media_id); return get(personal(`/media/${c.media_id}/related`));
        case "edit_media": {
          validId(c.media_id);
          if (!c.patch || typeof c.patch !== "object" || Array.isArray(c.patch) || !Object.keys(c.patch).every(k => MEDIA_FIELDS.includes(k))) throw invalid("Champs de fiche invalides.");
          return mutation(() => request("PUT", personal(`/media/${c.media_id}`), { body: { ...c.patch, expected_revision: c.expected_revision } }));
        }
        case "series": return get(personal(`/series/${segment(c.name)}`));
        case "edit_series":
          if (!c.patch || typeof c.patch !== "object" || !Object.keys(c.patch).every(k => SERIES_FIELDS.includes(k))) throw invalid("Champs communs de série invalides.");
          return mutation(() => request("PUT", personal(`/series/${segment(c.series)}`), { body: { expected_revision: c.expected_revision, patch: c.patch } }));
        case "sagas": return get(personal("/sagas"));
        case "saga": if (!SAGA_ID.test(c.id)) throw invalid("Identifiant de saga invalide."); return get(personal(`/sagas/${c.id}`));
        case "create_saga": return mutation(() => request("POST", personal("/sagas"), { body: { name: c.name } }));
        case "rename_saga": return mutation(() => request("PUT", personal(`/sagas/${c.id}`), { body: { expected_revision: c.expected_revision, name: c.name } }));
        case "add_saga": validMediaIds(c.media_ids); return mutation(() => request("POST", personal(`/sagas/${c.id}/members`), { body: { expected_revision: c.expected_revision, media_ids: c.media_ids } }));
        case "remove_saga_member": validId(c.media_id); return mutation(() => request("DELETE", personal(`/sagas/${c.id}/members`), { body: { expected_revision: c.expected_revision, media_ids: [c.media_id] } }));
        case "reorder_saga": validMediaIds(c.media_ids); return mutation(() => request("PUT", personal(`/sagas/${c.id}/order`), { body: { expected_revision: c.expected_revision, expected_order: c.expected_order, media_ids: c.media_ids } }));
        case "delete_saga": return mutation(() => request("DELETE", personal(`/sagas/${c.id}`), { body: { expected_revision: c.expected_revision } }));
        case "playlists": return get(personal("/playlists"));
        case "playlist": validId(c.id); return get(personal(`/playlists/${c.id}`));
        case "create_playlist": if (c.media_id) validId(c.media_id); return mutation(() => request("POST", personal("/playlists"), { body: { name: c.name, media_id: c.media_id ?? null } }));
        case "add_playlist": validId(c.id); validMediaIds(c.media_ids); return mutation(() => request("POST", personal(`/playlists/${c.id}/members`), { body: { expected_revision: c.expected_revision, media_ids: c.media_ids } }));
        case "remove_playlist_member": validId(c.id); validId(c.media_id); return mutation(() => request("DELETE", personal(`/playlists/${c.id}/members/${c.media_id}`), { body: { expected_revision: c.expected_revision } }));
        case "reorder_playlist": validId(c.id); validMediaIds(c.media_ids); return mutation(() => request("PUT", personal(`/playlists/${c.id}/order`), { body: { expected_revision: c.expected_revision, expected_order: c.expected_order, media_ids: c.media_ids } }));
        case "delete_playlist": validId(c.id); return mutation(() => request("DELETE", personal(`/playlists/${c.id}`)));
        case "rename_playlist": validId(c.id); return mutation(() => request("PUT", personal(`/playlists/${c.id}`), { body: { expected_revision: c.expected_revision, name: c.name } }));
        case "start_scan": return mutation(() => request("POST", "/admin/scan", { body: {} }));
        case "scan_status": return get("/admin/scan");
        case "artwork": validId(c.media_id); return artworkFor("media", c.media_id);
        case "saga_artwork": if (!SAGA_ID.test(c.id)) throw invalid("Identifiant de saga invalide."); return artworkFor("sagas", c.id);
        case "playlist_artwork": validId(c.id); return artworkFor("playlists", c.id);
        case "season_artwork":
          if (!Number.isSafeInteger(c.tv_id) || c.tv_id < 1 || !Number.isSafeInteger(c.season) || c.season < 0) throw invalid("Saison invalide.");
          return artworkFor(`series/${c.tv_id}/seasons/${c.season}`, null);
        case "save_player_settings":
          if (typeof c.fullscreen_on_start !== "boolean") throw invalid("Réglage du lecteur invalide.");
          settings.player_presentation = { fullscreen_on_start: c.fullscreen_on_start }; save();
          return { player_presentation: settings.player_presentation };
        case "theme":
          if (!["system", "light", "dark"].includes(c.theme)) throw invalid("Thème invalide.");
          settings.theme = c.theme; save(); return view();
        case "save_appearance": {
          const a = c.appearance || {};
          if (!["auto", "manual", "off"].includes(a.mode) || !["spring", "summer", "autumn", "winter", "christmas", "halloween"].includes(a.manual)) throw invalid("Apparence invalide.");
          settings.appearance = { mode: a.mode, manual: a.manual }; save(); return view();
        }
        case "resolve_appearance": return { ambiance: resolveAppearance(settings.appearance, c.year, c.month, c.day), appearance: settings.appearance };
        case "cache_settings": return { cache: settings.playback_cache };
        case "save_cache_settings": {
          const { forward_mib: f, back_mib: b } = c.cache || {};
          if (!Number.isInteger(f) || !Number.isInteger(b) || f < 16 || f > 512 || b < 0 || b > 256) throw invalid("Tampon avant : 16 à 512 Mio ; arrière : 0 à 256 Mio.");
          settings.playback_cache = { forward_mib: f, back_mib: b }; save(); return { cache: settings.playback_cache };
        }
        default: return unsupported();
      }
    }

    async function invoke(name, args) {
      if (name === "ui_command") return ui(args.command);
      if (name === "player_command") { if (!player) unsupported(); return player.command(args.command, conn && { profile: conn.active, request, base: BASE }); }
      if (name === "qa_result") return null;
      return unsupported();
    }
    return { invoke, _state: () => ({ settings, conn }) };
  }

  const api = { createBridge, resolveAppearance, catalogQuery, BASE };
  if (typeof module !== "undefined") module.exports = api;
  else {
    root.ChillBridge = api;
    if (!root.__TAURI__) {
      const bridge = createBridge({
        fetch: (...a) => root.fetch(...a), storage: root.localStorage, address: root.location.host,
        objectUrl: b => URL.createObjectURL(b), revoke: u => URL.revokeObjectURL(u),
        player: root.ChillWebPlayer || null,
      });
      root.__TAURI__ = { core: { invoke: bridge.invoke } };
    }
  }
})(typeof window !== "undefined" ? window : globalThis);
