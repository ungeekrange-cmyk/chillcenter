"use strict";

(() => {
  const label = text => window.chillI18n?.t(text) || text;
  const scope = () => state.profile_id && state.server_id ? state.server_id + "/" + state.profile_id : "";
  const web = Boolean(window.ChillBridge);
  const format = value => value == null ? label("Indisponible") : new Intl.NumberFormat(
    window.chillI18n?.language || "fr", { style: "unit", unit: value >= 1000000000 ? "gigabyte" : "megabyte", maximumFractionDigits: 1 },
  ).format(value / (value >= 1000000000 ? 1000000000 : 1000000));
  const sum = copies => copies.reduce((total, copy) => total + (copy.local_bytes || 0), 0);
  const control = (text, action) => {
    const element = node("button", label(text), "secondary");
    element.type = "button";
    element.onclick = action;
    return element;
  };
  const command = (action, fields = {}) => window.__TAURI__.core.invoke("downloads_command", { command: { action, ...fields } });
  const tab = control("Téléchargements", async () => {
    if (await stopPlayerForNavigation()) { shellShow("downloads"); await refresh(); }
  });
  tab.id = "show-downloads";
  tab.hidden = true;
  $("show-playlists").after(tab);
  const panel = node("section", undefined, "downloads-panel");
  panel.id = "downloads-panel";
  panel.hidden = true;
  $("catalog-panel").after(panel);
  const header = node("div", undefined, "section-head");
  const title = node("h1", label("Téléchargements"));
  const storage = node("p", "", "muted");
  const list = node("div", undefined, "downloads-list");
  const jobs = node("div", undefined, "downloads-jobs");
  const status = node("p", "", "muted");
  status.setAttribute("role", "status");
  const cancel = control("Annuler le téléchargement", () => guarded(() => command("cancel", { media_id: runningMedia })));
  cancel.hidden = true;
  const report = node("div", undefined, "download-notice");
  report.append(status, cancel);
  $("message").after(report);
  const configuration = node("section", undefined, "panel");
  configuration.dataset.settingsCategory = "downloads";
  configuration.id = "download-settings";
  configuration.append(node("h2", label("Téléchargements")));
  $("admin-panel").querySelector(".admin-grid").append(configuration);
  let items = [], runningMedia = null, ticket = 0, currentScope = "", busy = false, activeFeedback = null;
  const switches = [header, configuration].map(container => {
    const wrap = node("label", undefined, "download-preference");
    const input = node("input");
    input.type = "checkbox";
    input.checked = true;
    input.onchange = () => guarded(async () => {
      await command("set_auto", { enabled: input.checked });
      await refresh();
    }, refresh);
    wrap.append(input, node("span", label("Supprimer après visionnage")));
    container.append(wrap);
    return input;
  });
  header.prepend(title);
  if (!web) header.append(control("Ouvrir dans le Finder", () => guarded(() => command("open_folder"))));
  panel.append(header, storage, jobs, list);

  async function guarded(work, recover) {
    try { return await work(); }
    catch (error) {
      status.textContent = error?.message || String(error);
      await recover?.();
    }
  }

  function removeCopies(copies) {
    const space = format(sum(copies));
    const text = copies.length === 1 ? label("Supprimer le téléchargement de ") + copies[0].media.title + " (" + space + ") ?"
      : label("Supprimer les ") + copies.length + label(" téléchargements sélectionnés") + " (" + space + ") ?";
    if (window.confirm(text)) void guarded(async () => {
      await command("delete", { ids: copies.map(copy => copy.media.id) });
      await refresh();
    });
  }

  function video(copy) {
    const row = node("article", undefined, "download-row");
    const { media } = copy;
    const title = node("strong", (media.episode == null ? "" : "E" + media.episode + " · ") + media.title);
    const detail = node("span", format(copy.local_bytes), "muted");
    const play = control("Lire", () => window.chillPlay(media.id, null, true));
    play.disabled = Boolean(copy.access_error);
    const keep = control(copy.keep ? "Autoriser la suppression automatique" : "Conserver cette vidéo",
      () => guarded(async () => { await command("keep", { media_id: media.id, enabled: !copy.keep }); await refresh(); }));
    keep.setAttribute("aria-pressed", String(Boolean(copy.keep)));
    if (copy.has_poster) {
      const image = node("img", undefined, "download-poster");
      image.alt = "";
      image.hidden = true;
      row.append(image);
      void command("artwork", { media_id: media.id }).then(asset => {
        if (!asset?.src) return;
        const revoke = () => { if (web) URL.revokeObjectURL(asset.src); };
        if (!image.isConnected) { revoke(); return; }
        image.onload = () => { image.hidden = false; revoke(); };
        image.onerror = revoke;
        image.src = asset.src;
      }).catch(() => {});
    }
    row.append(title, detail, play, keep, control("Supprimer le téléchargement", () => removeCopies([copy])));
    if (copy.access_error) row.append(node("p", label("Fichier local inaccessible."), "muted"));
    if (copy.missing_subtitles) {
      row.append(node("p", label("Certains sous-titres externes sont indisponibles."), "muted"));
      if (!state.downloads_only) row.append(control("Réessayer les sous-titres", () => void download(media, "video")));
    }
    if (copy.deletion_failed) row.append(node("p", label("Suppression à réessayer."), "muted"));
    return row;
  }

  function group(name, copies, entries) {
    const section = node("details", undefined, "download-group");
    section.dataset.downloadGroup = name + "|" + copies[0].media.id;
    section.append(node("summary", name + " · " + format(sum(copies))));
    section.append(control("Supprimer les téléchargements du groupe", () => removeCopies(copies)), ...entries);
    return section;
  }

  function renderList() {
    const opened = new Set(Array.from(list.querySelectorAll("details[open]")).map(el => el.dataset.downloadGroup));
    const films = [], series = new Map();
    for (const copy of items) {
      if (copy.media.category !== "serie") films.push(copy);
      else {
        const key = copy.series_key || copy.media.series || copy.media.id;
        if (!series.has(key)) series.set(key, []);
        series.get(key).push(copy);
      }
    }
    const entries = films.map(video);
    for (const episodes of series.values()) {
      const seasons = new Map();
      for (const episode of episodes) {
        const season = episode.media.season ?? null;
        if (!seasons.has(season)) seasons.set(season, []);
        seasons.get(season).push(episode);
      }
      const children = [...seasons].sort((a, b) => (a[0] ?? Infinity) - (b[0] ?? Infinity)).map(([number, season]) =>
        group(number === null ? label("Saison inconnue") : label("Saison ") + number, season,
          season.sort((a, b) => (a.media.episode ?? Infinity) - (b.media.episode ?? Infinity)).map(video)));
      entries.push(group(episodes[0].media.series || episodes[0].media.title, episodes, children));
    }
    list.replaceChildren(...entries);
    for (const element of list.querySelectorAll("details")) element.open = opened.has(element.dataset.downloadGroup);
  }

  async function refresh() {
    const key = scope(), token = ++ticket;
    if (!key) return visibility();
    const result = await command("list").catch(error => {
      if (key === currentScope && shellSection === "downloads") status.textContent = error.message;
      return null;
    });
    if (!result || token !== ticket || key !== scope()) return;
    const previous = items;
    items = result.items || [];
    configuration.hidden = !result.supported;
    for (const input of switches) input.checked = result.enabled !== false;
    storage.textContent = label("Téléchargements") + " · " + format(result.total || 0) + "  |  " + format(result.free) + " " + label("libres sur") + " " + format(result.capacity) + (web ? " (" + label("estimation du navigateur") + ")" : "");
    const notices = [];
    let transferring = false;
    for (const job of result.jobs || []) {
      const row = node("div", undefined, "download-row");
      row.append(node("strong", job.media.title), node("span", format(job.transferred) + " / " + format(job.bytes)));
      if (job.status === "downloading") {
        transferring = true;
        runningMedia = job.media.id;
        cancel.hidden = false;
        const progress = node("progress");
        progress.max = job.bytes || 1;
        progress.value = job.transferred || 0;
        row.append(progress);
        if (job.media.title) reportDownload(job.media.title + " · " + format(job.transferred) + " / " + format(job.bytes));
      } else if (!state.downloads_only) {
        row.append(control("Reprendre le téléchargement", () => void download(job.media, "video")));
      }
      notices.push(row);
    }
    if (!web && !transferring && !busy) cancel.hidden = true;
    jobs.replaceChildren(...notices);
    if (JSON.stringify(previous) !== JSON.stringify(items) || !list.childNodes.length) renderList();
    visibility();
  }

  function visibility() {
    const active = Boolean(scope());
    tab.hidden = !active || !items.length;
    tab.classList.toggle("active", shellSection === "downloads");
    panel.hidden = !active || shellSection !== "downloads";
    if (!panel.hidden && !items.length && !list.childNodes.length) {
      list.append(node("p", label("Aucun téléchargement sur cet appareil."), "muted"));
    }
  }

  function downloadFeedback() {
    const feedback = node("p", "", "download-feedback");
    feedback.hidden = true;
    feedback.setAttribute("role", "status");
    return feedback;
  }

  function reportDownload(text, feedback = activeFeedback) {
    status.textContent = text;
    if (feedback?.isConnected) {
      feedback.textContent = text;
      feedback.hidden = !text;
    }
  }

  function showHttpsHelp(feedback) {
    const text = label("Ouvre Chill Center en HTTPS pour télécharger.");
    if (!feedback?.isConnected) feedback = status;
    reportDownload(text, feedback);
    const link = node("a", label("Ouvrir la page HTTPS"));
    const url = new URL(window.location.origin);
    url.protocol = "https:";
    url.port = "8443";
    link.href = url.href;
    feedback.append(" ", link);
  }

  async function download(media, selection, feedback) {
    if (web && !window.isSecureContext) { showHttpsHelp(feedback); return; }
    if (busy) return;
    busy = true;
    const key = scope();
    try {
      const plan = await command("plan", { media_id: media.id, scope: selection });
      if (key !== scope()) return;
      if (!plan.items?.length) { reportDownload(label("Aucune vidéo disponible à télécharger."), feedback); return; }
      const total = plan.items.reduce((size, item) => size + item.bytes, 0);
      if (selection !== "video") {
        const summary = plan.items.length + " " + label("vidéos à télécharger") + " · " + format(total)
          + (plan.skipped ? " · " + plan.skipped + " " + label("indisponibles") : "");
        if (!window.confirm(summary)) return;
      }
      activeFeedback = feedback;
      runningMedia = media.id;
      cancel.hidden = false;
      reportDownload(label("Préparation du téléchargement…"));
      await command("start", { media_id: media.id, scope: selection });
      await refresh();
    } catch (error) { cancel.hidden = true; reportDownload(error.message || String(error), feedback); }
    finally { busy = false; }
  }

  window.createDownloadFeedback = downloadFeedback;
  window.offlineDownloadButton = (media, selection = "video", text = "Télécharger", feedback) => {
    if (!media || !state.offline_downloads || state.downloads_only) return null;
    const button = control(text, () => void download(media, selection, feedback));
    button.disabled = selection === "video" && media.available === false;
    return button;
  };
  window.renderDownloads = visibility;
  window.refreshDownloads = refresh;
  window.downloadsContextChanged = () => {
    if (scope() !== currentScope) {
      currentScope = scope();
      items = [];
      list.replaceChildren();
      jobs.replaceChildren();
      status.textContent = "";
      activeFeedback = null;
      cancel.hidden = true;
      if (state.downloads_only) shellSection = "downloads";
      if (currentScope) void refresh();
    }
    configuration.querySelector("label").hidden = !currentScope;
    visibility();
  };
  window.addEventListener("chill:downloads", () => void refresh());
  window.addEventListener("chill:download-result", ({ detail }) => {
    cancel.hidden = true;
    reportDownload(detail?.errors?.join(" · ") || label("Téléchargements à jour."));
    activeFeedback = null;
    void refresh();
  });
  if (!web) void window.__TAURI__?.event?.listen?.("chill:downloads", () => void refresh());
  if (!web) void window.__TAURI__?.event?.listen?.("chill:download-result", ({ payload }) =>
    window.dispatchEvent(new CustomEvent("chill:download-result", { detail: payload })));
  window.addEventListener("focus", () => void refresh());
  setInterval(() => {
    if (scope() && (shellSection === "downloads" || !cancel.hidden)) void refresh();
  }, 3000);
})();
