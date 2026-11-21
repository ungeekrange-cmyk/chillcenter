"use strict";

(function (root) {
  const BASE = "/api/2026-10-14";
  const VERSION = /^["a-zA-Z0-9.:_-]{1,120}$/;
  const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const CHUNK = 1024 * 1024;
  const MAX_SUBTITLE = 16 * CHUNK;
  const MAX_POSTER = 2 * CHUNK;
  const POSTER_TYPES = new Set(["image/jpeg", "image/png", "image/webp"]);
  const changed = () => root.dispatchEvent(new Event("chill:downloads"));
  const safeContext = ({ server, profile } = {}) => {
    if (!UUID.test(server || "") || !UUID.test(profile || "")) throw new Error("Profil hors connexion indisponible.");
    return server.toLowerCase() + "-" + profile.toLowerCase();
  };

  function indexedStorage(idb) {
    let opening;
    const open = () => opening ||= new Promise((resolve, reject) => {
      const request = idb.open("chillcenter-downloads", 1);
      request.onupgradeneeded = () => {
        const db = request.result;
        db.createObjectStore("copies", { keyPath: "key" }).createIndex("scope", "scope");
        db.createObjectStore("preferences", { keyPath: "key" });
        db.createObjectStore("progress", { keyPath: "key" }).createIndex("scope", "scope");
      };
      request.onsuccess = () => {
        request.result.onversionchange = () => { request.result.close(); opening = null; };
        resolve(request.result);
      };
      request.onerror = () => { opening = null; reject(request.error); };
      request.onblocked = () => { opening = null; reject(new Error("Stockage bloqué dans un autre onglet.")); };
    });
    return async (table, mode, operation) => {
      const db = await open();
      const tx = db.transaction(table, mode);
      return new Promise((resolve, reject) => {
        const result = operation(tx.objectStore(table));
        tx.oncomplete = () => resolve(result.result);
        tx.onabort = tx.onerror = () => reject(tx.error || result.error || new Error("Stockage indisponible."));
      });
    };
  }

  function writer(url) {
    const worker = new Worker(url);
    let current, sequence = 0, failed;
    worker.onmessage = ({ data }) => {
      if (data.id !== current?.id) return;
      const job = current;
      current = null;
      if (data.error) job.reject(Object.assign(new Error(data.error), { name: data.name }));
      else job.resolve(data.value);
    };
    worker.onerror = () => {
      failed = new Error("Écriture locale indisponible.");
      current?.reject(failed);
      current = null;
    };
    return {
      call(action, fields = {}, transfer = []) {
        if (failed) return Promise.reject(failed);
        return new Promise((resolve, reject) => {
          const id = ++sequence;
          current = { id, resolve, reject };
          worker.postMessage({ id, action, ...fields }, transfer);
        });
      },
      terminate() { worker.terminate(); },
    };
  }

  const versionedUrl = "offline-writer.js" + (root.document?.currentScript ? new URL(root.document.currentScript.src).search : "");
  function createDownloads({ storage, idb, locks, fetch, online, prepare, writerFactory, notify }) {
    const record = idb && indexedStorage(idb);
    let active = null;
    const queuedDeletes = new Set();
    const supported = () => Boolean(storage?.getDirectory && locks && record);
    const requireLocal = () => {
      if (!supported()) throw new Error("Le stockage hors connexion n’est pas disponible dans ce navigateur.");
    };
    const base = context => {
      safeContext(context);
      return BASE + "/profiles/" + context.profile + "/downloads";
    };
    const keyFor = (context, id) => {
      if (!UUID.test(id || "")) throw new Error("Vidéo inconnue.");
      return safeContext(context) + "-" + id.toLowerCase();
    };
    const get = (table, key) => record(table, "readonly", store => store.get(key));
    const put = (table, value) => record(table, "readwrite", store => store.put(value));
    const remove = (table, key) => record(table, "readwrite", store => store.delete(key));
    const byScope = (table, scope) => record(table, "readonly", store => store.index("scope").getAll(scope));
    const directory = async () => (await storage.getDirectory()).getDirectoryHandle("chillcenter-downloads", { create: true });
    const copyFolder = async key => (await directory()).getDirectoryHandle(key);
    const findFile = async (folder, name) => (await folder.getFileHandle(name)).getFile();
    const lockName = key => "chillcenter-copy:" + key;
    const request = (path, options = {}) => fetch(path, { credentials: "same-origin", cache: "no-store", ...options });
    const responseJson = async (path, options = {}) => {
      const response = await request(path, options);
      if (!response.ok) {
        const details = await response.json().catch(() => ({}));
        throw Object.assign(new Error(details.message || "Téléchargement indisponible."), { code: details.code, status: response.status });
      }
      return response.json();
    };
    const preference = async context => (await get("preferences", safeContext(context)))?.enabled !== false;

    async function list(context) {
      if (!supported()) return { items: [], jobs: [], total: 0, enabled: true, supported: false };
      const scope = safeContext(context);
      const rows = await byScope("copies", scope);
      for (const progress of await byScope("progress", scope)) {
        if (progress.delete_instance) queueDelete(context, progress.id, progress.delete_instance);
      }
      const items = [], jobs = [];
      for (const row of rows) {
        if (!row.ready) {
          jobs.push({ ...row, status: active?.scope === scope && active?.id === row.media.id ? "downloading" : "interrupted" });
          continue;
        }
        try {
          const folder = await copyFolder(row.fileKey);
          const video = await findFile(folder, "video");
          if (video.size !== row.bytes) continue;
          let total = video.size;
          for (const sidecar of row.subtitles) {
            try { total += (await findFile(folder, sidecar.id)).size; }
            catch (error) { if (error?.name !== "NotFoundError") throw error; }
          }
          if (row.posterType) {
            try { total += (await findFile(folder, "poster")).size; }
            catch (error) { if (error?.name !== "NotFoundError") throw error; }
          }
          items.push({ ...row, local_bytes: total, has_poster: Boolean(row.posterType) });
        } catch (error) {
          if (error?.name !== "NotFoundError") items.push({ ...row, local_bytes: row.bytes, access_error: true });
        }
      }
      const estimate = await storage.estimate?.().catch(() => null);
      return {
        items, jobs, total: items.reduce((sum, item) => sum + item.local_bytes, 0), supported: true,
        busy: active?.scope === scope,
        capacity: Number.isFinite(estimate?.quota) ? estimate.quota : null,
        free: Number.isFinite(estimate?.quota) && Number.isFinite(estimate?.usage) ? Math.max(0, estimate.quota - estimate.usage) : null,
        enabled: await preference(context),
      };
    }

    async function plan(context, id, scope) {
      requireLocal();
      if (!online()) throw new Error("Ouvrez Chill Center en HTTPS reconnu pour préparer les téléchargements.");
      keyFor(context, id);
      if (!["video", "season", "series"].includes(scope)) throw new Error("Sélection inconnue.");
      const items = [];
      let offset = 0, skipped = 0;
      do {
        const page = await responseJson(base(context) + "/plan?" + new URLSearchParams({ media_id: id, scope, offset }));
        if (!Array.isArray(page.items) || items.length + page.items.length > 3000 || offset > 3000) throw new Error("Sélection trop importante.");
        items.push(...page.items);
        skipped += Number(page.skipped || 0);
        if (page.next_offset != null && (!Number.isSafeInteger(page.next_offset) || page.next_offset <= offset)) throw new Error("Page invalide.");
        offset = page.next_offset;
      } while (offset != null);
      return { items, skipped };
    }

    async function boundedBytes(response, limit, exact) {
      if (!response.body) throw new Error("Réponse vide.");
      if (Number(response.headers.get("content-length")) > limit) {
        await response.body.cancel();
        throw new Error("Réponse trop volumineuse.");
      }
      const reader = response.body.getReader();
      const result = new Uint8Array(limit);
      let size = 0;
      try {
        for (;;) {
          const { value, done } = await reader.read();
          if (done) break;
          if (value.byteLength > limit - size) throw new Error("Réponse trop volumineuse.");
          result.set(value, size);
          size += value.byteLength;
        }
        if (exact && size !== limit) throw new Error("Réponse incomplète.");
        return size === limit ? result : result.slice(0, size);
      } finally { await reader.cancel().catch(() => {}); reader.releaseLock(); }
    }

    async function one(context, item, job) {
      const { media, version, bytes } = item;
      if (!VERSION.test(version || "") || !Number.isSafeInteger(bytes) || bytes <= 0) throw new Error("Taille ou version de téléchargement invalide.");
      const key = keyFor(context, media?.id), scope = safeContext(context);
      const url = base(context) + "/" + media.id;
      await locks.request(lockName(key), { signal: job.controller.signal }, async () => {
        const previous = await get("copies", key);
        const fileKey = key + "-" + version.replace(/[^a-z0-9_-]/gi, "_");
        if (previous?.version === version && previous.ready && !previous.missing_subtitles) {
          try { if ((await findFile(await copyFolder(previous.fileKey), "video")).size === bytes) return; }
          catch (error) { if (error?.name !== "NotFoundError") throw error; }
        }
        if (previous && !previous.ready && previous.fileKey !== fileKey) await eraseFolder(previous.fileKey);
        const session = await responseJson(url + "/session", {
          method: "POST", headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ version, device_id: job.device }), signal: job.controller.signal,
        });
        const headers = { "X-Chill-Download": session.token };
        const signal = job.controller.signal;
        let stream;
        try {
          if (session.version !== version || session.bytes !== bytes) throw new Error("La vidéo a changé. Préparez un nouveau téléchargement.");
          const row = {
            ...item, key, fileKey, scope, ready: false, transferred: 0, keep: previous?.keep === true, subtitles: [],
            preferences: session.preferences || null, personal: session.personal || null, progress_revision: session.progress_revision || null,
          };
          const update = async () => { if (!previous?.ready) await put("copies", row); notify(); };
          stream = writerFactory();
          let position = await stream.call("open", { key: fileKey, name: "video" });
          if (!Number.isSafeInteger(position) || position < 0 || position > bytes) {
            await stream.call("truncate");
            position = 0;
          }
          row.transferred = position;
          await update();
          const estimate = await storage.estimate?.().catch(() => null);
          if (Number.isFinite(estimate?.quota) && Number.isFinite(estimate?.usage) && estimate.quota - estimate.usage < bytes - position) {
            throw new Error("L’espace disponible dans le navigateur est insuffisant.");
          }
          while (position < bytes) {
            const end = Math.min(position + CHUNK, bytes) - 1;
            const range = position + "-" + end;
            const response = await request(url + "/content", { headers: { ...headers, Range: "bytes=" + range, "If-Match": version }, signal });
            if (response.status !== 206 || response.headers.get("content-range") !== "bytes " + range + "/" + bytes) {
              await response.body?.cancel().catch(() => {});
              throw new Error("Plage vidéo refusée. Réessayez le téléchargement.");
            }
            const length = end - position + 1;
            const chunk = await boundedBytes(response, length, true);
            await stream.call("write", { offset: position, bytes: chunk.buffer }, [chunk.buffer]);
            position += length;
            row.transferred = position;
            if (position === bytes || position % (8 * CHUNK) === 0) await update();
          }
          await stream.call("close");
          let missing = 0;
          for (const subtitle of (session.subtitles || []).slice(0, 64)) {
            if (!UUID.test(subtitle.id) || !/\.(srt|vtt|ass|ssa)$/i.test(subtitle.title)) continue;
            try {
              const response = await request(url + "/subtitles/" + subtitle.id, { headers, signal });
              if (!response.ok) throw new Error("Sous-titre indisponible.");
              const known = subtitle.bytes > 0 && subtitle.bytes <= MAX_SUBTITLE;
              const content = await boundedBytes(response, known ? subtitle.bytes : MAX_SUBTITLE, known);
              await stream.call("open", { key: fileKey, name: subtitle.id });
              await stream.call("truncate");
              await stream.call("write", { offset: 0, bytes: content.buffer }, [content.buffer]);
              await stream.call("close");
              row.subtitles.push({ ...subtitle, unavailable: false });
            } catch (error) {
              await stream.call("close").catch(() => {});
              if (signal.aborted) throw error;
              missing++;
            }
          }
          if (signal.aborted) throw signal.reason;
          if (media.poster) {
            try {
              const response = await request(BASE + "/profiles/" + context.profile + "/media/" + media.id + "/artwork", { signal });
              const type = response.headers.get("content-type");
              if (response.ok && POSTER_TYPES.has(type)) {
                const content = await boundedBytes(response, MAX_POSTER, false);
                if (content.byteLength) {
                  await stream.call("open", { key: fileKey, name: "poster" });
                  await stream.call("truncate");
                  await stream.call("write", { offset: 0, bytes: content.buffer }, [content.buffer]);
                  await stream.call("close");
                  row.posterType = type;
                }
              }
            } catch (error) {
              await stream.call("close").catch(() => {});
              if (signal.aborted) throw error;
            }
          }
          row.missing_subtitles = missing;
          row.ready = true;
          row.instance = root.crypto.randomUUID();
          await put("copies", row);
          if (previous && previous.fileKey !== fileKey) await eraseFolder(previous.fileKey).catch(() => {});
          notify();
        } finally {
          stream?.terminate();
          await request(url + "/session", { method: "DELETE", headers, signal: AbortSignal.timeout(3000) }).catch(() => {});
        }
      });
    }

    async function eraseFolder(key) {
      try { await (await directory()).removeEntry(key, { recursive: true }); }
      catch (error) { if (error?.name !== "NotFoundError") throw error; }
    }
    const progressLock = key => "chillcenter-download-progress:" + key;

    function queueDelete(context, id, instance) {
      if (!instance || queuedDeletes.has(instance)) return;
      queuedDeletes.add(instance);
      void erase(context, id, instance).catch(() => {}).finally(() => queuedDeletes.delete(instance));
    }

    async function erase(context, id, instance = null) {
      const key = keyFor(context, id);
      return locks.request(lockName(key), async () => {
        const row = await get("copies", key);
        const unlink = async () => {
          if (!row) return;
          try {
            await eraseFolder(row.fileKey);
            await remove("copies", key);
          } catch (error) {
            if (instance) await put("copies", { ...row, deletion_failed: true });
            throw error;
          } finally { notify(); }
        };
        if (!instance) return unlink();
        try {
          await locks.request("chillcenter-download-policy:" + safeContext(context), async () => {
            if (row?.instance === instance && !row.keep && await preference(context)) await unlink();
          });
        } finally {
          await locks.request(progressLock(key), async () => {
            const current = await get("progress", key);
            if (current?.delete_instance === instance) await put("progress", { ...current, delete_instance: null });
          });
        }
      });
    }

    async function acquire(context, id) {
      requireLocal();
      const key = keyFor(context, id);
      let unlock;
      const held = new Promise(resolve => { unlock = resolve; });
      return new Promise((resolve, reject) => {
        locks.request(lockName(key), { mode: "shared" }, async () => {
          const row = await get("copies", key);
          if (!row?.ready) throw new Error("Copie locale indisponible.");
          const folder = await copyFolder(row.fileKey);
          const file = await findFile(folder, "video");
          if (file.size !== row.bytes) throw new Error("Copie locale incomplète.");
          const progress = await get("progress", key);
          const subtitles = [];
          for (const sub of row.subtitles) {
            try { await findFile(folder, sub.id); subtitles.push(sub); }
            catch (error) { if (error?.name !== "NotFoundError") throw error; }
          }
          resolve({ row, progress, file, subtitles, readSubtitle: async id => {
            if (!subtitles.some(s => s.id === id)) throw new Error("Piste locale inconnue.");
            const data = await findFile(folder, id);
            if (data.size > MAX_SUBTITLE) throw new Error("Sous-titre local trop volumineux.");
            return data.text();
          }, close: unlock });
          await held;
        }).catch(reject);
      });
    }

    async function saveProgress(context, id, event, completed, revision, pending, copy) {
      const scope = safeContext(context), key = keyFor(context, id);
      await locks.request(progressLock(key), async () => {
        const previous = await get("progress", key);
        const current = completed && await get("copies", key);
        const eligible = current?.instance && current.instance === copy?.instance && !current.keep && await preference(context);
        const delete_instance = eligible ? copy.instance : previous?.delete_instance || null;
        await put("progress", {
          key, scope, id, position_ms: event.position_ms, duration_ms: event.duration_ms, watched: completed,
          base_revision: revision || null, event_id: root.crypto.randomUUID(), pending, delete_instance,
        });
        if (delete_instance) queueDelete(context, id, delete_instance);
      });
    }

    async function sync(context) {
      if (!supported() || !context?.server) return;
      const scope = safeContext(context);
      await locks.request("chillcenter-download-sync:" + scope, { ifAvailable: true }, async lock => {
        if (!lock) return;
        const pending = (await byScope("progress", scope)).filter(row => row.pending && row.base_revision);
        if (!pending.length) return;
        const remote = await responseJson(BASE + "/server", { signal: AbortSignal.timeout(3500) });
        if (remote?.server_id !== context.server) return;
        for (const progress of pending) {
          const path = base(context) + "/" + progress.id + "/progress";
          try {
            const { event_id, base_revision, position_ms, duration_ms, watched } = progress;
            const result = await responseJson(path, {
              method: "PUT", headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ event_id, base_revision, position_ms, duration_ms, watched }),
              signal: AbortSignal.timeout(3500),
            });
            await locks.request(progressLock(progress.key), async () => {
              const latest = await get("progress", progress.key);
              if (latest?.event_id === event_id) await put("progress", { ...latest, pending: false, base_revision: result.revision });
            });
          } catch (error) {
            if (![404, 409].includes(error.status)) return;
          }
        }
      });
    }

    return { plan, list, acquire, saveProgress, sync,
      async command(command, context, device) {
        if (command.action === "list") return list(context);
        requireLocal();
        switch (command.action) {
          case "plan": return plan(context, command.media_id, command.scope);
          case "settings": return { enabled: await preference(context) };
          case "artwork": {
            const key = keyFor(context, command.media_id);
            return locks.request(lockName(key), { mode: "shared" }, async () => {
              const row = await get("copies", key);
              if (!row?.ready || !POSTER_TYPES.has(row.posterType)) return null;
              const file = await findFile(await copyFolder(row.fileKey), "poster");
              return file.size <= MAX_POSTER ? { src: URL.createObjectURL(new Blob([file], { type: row.posterType })) } : null;
            });
          }
          case "set_auto":
            return locks.request("chillcenter-download-policy:" + safeContext(context), async () => {
              const enabled = command.enabled === true;
              await put("preferences", { key: safeContext(context), enabled });
              notify();
              return { enabled };
            });
          case "keep": {
            const key = keyFor(context, command.media_id);
            await locks.request(lockName(key), { mode: "shared" }, async () => {
              const row = await get("copies", key);
              if (row?.ready) await put("copies", { ...row, keep: command.enabled === true });
            });
            notify();
            return {};
          }
          case "delete":
            for (const id of (command.ids || []).slice(0, 3000)) await erase(context, id);
            return {};
          case "cancel":
            if (active?.scope === safeContext(context)) active.controller.abort();
            return {};
          case "start": {
            if (!online()) throw new Error("Préparation hors ligne indisponible.");
            if (active) throw new Error("Un téléchargement est déjà en cours.");
            const controller = new AbortController();
            const job = { controller, scope: safeContext(context), id: command.media_id, device };
            active = job;
            void storage.persist?.().catch(() => {});
            void (async () => {
              const errors = [];
              try {
                await prepare();
                const result = await plan(context, command.media_id, command.scope);
                await locks.request("chillcenter-download-queue", { signal: controller.signal }, async () => {
                  for (const item of result.items) {
                    if (controller.signal.aborted) break;
                    job.id = item.media.id;
                    try { await one(context, item, job); }
                    catch (error) { if (!controller.signal.aborted) errors.push(item.media.title + " — " + error.message); }
                  }
                });
              } catch (error) { if (!controller.signal.aborted) errors.push(error.message); }
              finally { if (active === job) active = null; notify({ scope: job.scope, errors }); }
            })();
            return { started: true };
          }
          default: throw new Error("Commande de téléchargement inconnue.");
        }
      },
    };
  }

  if (typeof module !== "undefined") module.exports = { createDownloads };
  else {
    const channel = typeof BroadcastChannel === "function" ? new BroadcastChannel("chillcenter-downloads") : null;
    if (channel) channel.onmessage = changed;
    root.ChillWebDownloads = createDownloads({
      storage: root.navigator.storage, locks: root.navigator.locks, idb: root.indexedDB,
      fetch: (...args) => root.fetch(...args), writerFactory: () => writer(versionedUrl),
      online: () => root.isSecureContext && Boolean(root.navigator.serviceWorker?.controller),
      prepare: () => new Promise((resolve, reject) => {
        const channel = new MessageChannel();
        const timer = setTimeout(() => { channel.port1.close(); reject(new Error("Préparation hors connexion trop longue.")); }, 30000);
        channel.port1.onmessage = ({ data }) => {
          clearTimeout(timer);
          channel.port1.close();
          if (data?.ready) resolve();
          else reject(new Error("Impossible de mettre les codecs en cache."));
        };
        root.navigator.serviceWorker.controller.postMessage("prepare-downloads", [channel.port2]);
      }),
      notify(detail) {
        changed();
        channel?.postMessage("changed");
        if (detail) root.dispatchEvent(new CustomEvent("chill:download-result", { detail }));
      },
    });
  }
})(typeof window !== "undefined" ? window : globalThis);
