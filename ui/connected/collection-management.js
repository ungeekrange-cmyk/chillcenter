"use strict";
// One explicit editing surface for shared sagas and profile-owned playlists.
// Every asynchronous response belongs to the original server/profile/dialog.
(() => {
  let editor = null, deletion = null, cover = null, sequence = 0;
  const t = text => uiText(text);
  const dialogs = ["collection-settings-dialog", "collection-delete-confirm", "collection-cover-dialog"];
  function target(view, kind, epoch = collectionEpoch, context = collectionScope()) { return { view, kind, epoch, context }; }
  function current(x) { return Boolean(x && x.context === collectionScope() && x.epoch === collectionEpoch); }
  function closeDialog(id) { const dialog = $(id); if (dialog.open) dialog.close(); }
  function close() { ++sequence; cover = editor = deletion = null; for (const id of dialogs) closeDialog(id); }
  function setBusy(dialog, busy) { dialog.setAttribute("aria-busy", String(busy)); for (const control of dialog.querySelectorAll("button,input")) control.disabled = busy; }
  function recoveryTarget(x, status) {
    const id = x === cover ? "collection-cover-dialog" : x === editor ? "collection-settings-dialog" : "collection-delete-confirm";
    const owns = () => current(x) && $(id).open && [cover, editor, deletion].includes(x);
    return { root: $(id), label: "Actualiser le groupe", current: owns, identityCurrent: owns, async reload() {
      await window.recoveryRefreshCatalogue?.(); if (!owns()) return false;
      const result = await invoke({ action: x.kind === "saga" ? "saga" : "playlist", id: x.view.id });
      if (!owns() || result?.discarded) return false;
      Object.assign(x.view, result.saga || result.playlist);
      if (x === cover) { x.selection = null; ++x.generation; if (x.tab === "members") renderMembers(); updateSelection(); }
      if (x === deletion) $("collection-delete-name").textContent = x.view.name;
      status.textContent = t("Données actualisées. Vérifiez votre choix avant de confirmer."); return true;
    } };
  }
  async function mutate(command, x, status) {
    if (!current(x)) return null;
    const recovery = recoveryTarget(x, status);
    try {
      const result = await invoke(command);
      if (!result || result.discarded) return null;
      await syncAfterMutation();
      return current(x) ? result : null;
    } catch (error) {
      if (current(x)) {
        status.textContent = error.message || t("Modification non enregistrée. Réessayez.");
        window.offerConflictRecovery?.(error, recovery);
      }
      return null;
    }
  }
  function addTileActions(card, view, kind, epoch, context) {
    const actions = node("div", undefined, "collection-tile-actions");
    actions.append(button(t("Modifier"), async () => {
      if (!collectionCurrent(epoch, context)) return;
      await openCollection(view.id, kind);
      if (collectionView?.id === view.id && context === collectionScope()) openEditor();
    }), button(t("Supprimer…"), () => {
      if (collectionCurrent(epoch, context)) openDelete(target(view, kind, epoch, context));
    }, "secondary danger"));
    card.append(actions);
  }
  function render(view, kind, epoch, context) {
    close();
    const preview = $("saga-artwork-preview");
    retireArtwork(preview); preview.classList.remove("collection-mosaic");
    preview.replaceChildren(); preview.textContent = view.name.slice(0, 1).toUpperCase();
    collectionMosaic(preview, view, kind, epoch, context);
    $("collection-manage").onclick = () => openEditor();
    $("collection-delete").onclick = () => openDelete(target(view, kind));
    $("collection-artwork").onclick = () => openCover();
  }
  function openEditor() {
    if (!collectionView || collectionKind === "series") return;
    editor = target(collectionView, collectionKind);
    $("collection-settings-title").textContent = t(editor.kind === "saga" ? "Renommer la saga" : "Renommer la liste de lecture");
    $("collection-name").value = editor.view.name;
    $("collection-settings-status").textContent = "";
    setBusy($("collection-settings-dialog"), false); $("collection-settings-dialog").showModal(); $("collection-name").focus();
  }
  $("collection-rename").onsubmit = async event => {
    event.preventDefault(); const x = editor, dialog = $("collection-settings-dialog"), status = $("collection-settings-status");
    if (!current(x) || x.saving) return;
    const name = $("collection-name").value.trim(); if (!name) return;
    x.saving = true; setBusy(dialog, true); status.textContent = t("Enregistrement…");
    const result = await mutate({ action: x.kind === "saga" ? "rename_saga" : "rename_playlist", id: x.view.id, expected_revision: x.view.revision, name }, x, status);
    if (editor !== x) return;
    x.saving = false; setBusy(dialog, false);
    if (result && editor === x) { closeDialog(dialog.id); await openCollection(x.view.id, x.kind); }
  };
  $("collection-settings-close").onclick = $("collection-rename-cancel").onclick = () => closeDialog("collection-settings-dialog");
  function openDelete(x) {
    if (!current(x)) return;
    deletion = x; $("collection-delete-name").textContent = x.view.name;
    $("collection-delete-title").textContent = t(x.kind === "saga" ? "Supprimer cette saga ?" : "Supprimer cette liste de lecture ?");
    $("collection-delete-status").textContent = "";
    setBusy($("collection-delete-confirm"), false); $("collection-delete-confirm").showModal(); $("collection-delete-no").focus();
  }
  $("collection-delete-no").onclick = () => closeDialog("collection-delete-confirm");
  $("collection-delete-yes").onclick = async () => {
    const x = deletion, dialog = $("collection-delete-confirm"), status = $("collection-delete-status");
    if (!current(x) || x.saving) return;
    x.saving = true; setBusy(dialog, true); status.textContent = t("Suppression…");
    const command = { action: x.kind === "saga" ? "delete_saga" : "delete_playlist", id: x.view.id };
    if (x.kind === "saga") command.expected_revision = x.view.revision;
    const result = await mutate(command, x, status);
    if (deletion !== x) return;
    x.saving = false; setBusy(dialog, false);
    if (result && deletion === x) { closeDialog(dialog.id); collectionView = null; await openCollections(x.kind); }
  };
  function chooseTab(name) {
    if (!cover || cover.saving) return;
    cover.tab = name;
    for (const key of ["members", "tmdb", "import"]) {
      $("cover-" + key + "-panel").hidden = name !== key;
      $("cover-" + key + "-tab").setAttribute("aria-selected", String(name === key));
    }
    cover.selection = null; updateSelection();
    if (name === "members") renderMembers();
    if (name === "tmdb") $("cover-tmdb-query").focus();
  }
  function openCover() {
    if (!collectionView || collectionKind === "series") return;
    cover = { ...target(collectionView, collectionKind), generation: ++sequence, tab: "members", limit: 48, selection: null };
    $("collection-cover-name").textContent = cover.view.name;
    $("cover-members-query").value = ""; $("cover-tmdb-query").value = cover.view.name;
    $("cover-tmdb-grid").replaceChildren(); $("collection-cover-status").textContent = "";
    setBusy($("collection-cover-dialog"), false); $("collection-cover-dialog").showModal(); chooseTab("members");
  }
  function coverCurrent(x) { return cover === x && current(x) && $("collection-cover-dialog").open; }
  function updateSelection() {
    const selection = cover?.selection;
    $("cover-apply").disabled = !selection || Boolean(cover?.saving);
    $("cover-apply").hidden = cover?.tab === "import";
    $("cover-selection-label").textContent = selection ? selection.title : t("Sélectionnez une jaquette pour continuer.");
    for (const card of $("collection-cover-dialog").querySelectorAll(".cover-choice")) card.setAttribute("aria-pressed", String(card.dataset.choice === selection?.key));
  }
  function select(x, selection) { if (coverCurrent(x) && !x.saving) { x.selection = selection; updateSelection(); } }
  function choice(title, subtitle, key, activate) {
    const card = button("", activate, "cover-choice"), poster = node("div", title.slice(0, 1).toUpperCase(), "poster");
    card.dataset.choice = key; card.setAttribute("aria-pressed", "false"); card.setAttribute("aria-label", title);
    card.append(poster, node("strong", title), node("span", subtitle || "", "muted")); return { card, poster };
  }
  function renderMembers() {
    const x = cover; if (!coverCurrent(x) || x.tab !== "members") return;
    const grid = $("cover-members-grid"), query = $("cover-members-query").value.trim().toLocaleLowerCase();
    for (const poster of grid.querySelectorAll(".poster")) retireArtwork(poster);
    grid.replaceChildren();
    const items = x.view.media_ids.map(id => collectionDetails.get(id)).filter(m => m?.poster && (!query || m.title.toLocaleLowerCase().includes(query)));
    for (const media of items.slice(0, x.limit)) {
      const key = "member:" + media.id;
      const { card, poster } = choice(media.title, media.year ? String(media.year) : "", key, () => select(x, { key, type: "member", media_id: media.id, title: media.title }));
      grid.append(card); observeArtwork(poster, media.id);
    }
    $("cover-members-more").hidden = items.length <= x.limit;
    if (!items.length) grid.append(node("p", collectionLoaded < x.view.media_ids.length ? t("Chargement des jaquettes…") : t("Aucune jaquette disponible. Essayez une recherche TMDB ou importez une image."), "muted"));
    updateSelection();
  }
  $("cover-members-query").oninput = () => { if (cover) { cover.limit = 48; renderMembers(); } };
  $("cover-members-more").onclick = () => { if (cover) { cover.limit += 48; renderMembers(); } };
  for (const name of ["members", "tmdb", "import"]) {
    const tab = $("cover-" + name + "-tab"); tab.onclick = () => chooseTab(name);
    tab.onkeydown = event => { if (!["ArrowLeft", "ArrowRight"].includes(event.key)) return; event.preventDefault(); const keys = ["members", "tmdb", "import"], index = keys.indexOf(name), next = keys[(index + (event.key === "ArrowRight" ? 1 : 2)) % 3]; chooseTab(next); $("cover-" + next + "-tab").focus(); };
  }
  $("cover-tmdb-search").onsubmit = async event => {
    event.preventDefault(); const x = cover; if (!coverCurrent(x) || x.saving) return;
    const query = $("cover-tmdb-query").value.trim(); if (!query) return;
    const generation = ++x.generation, status = $("collection-cover-status"), grid = $("cover-tmdb-grid");
    x.selection = null; updateSelection(); grid.replaceChildren(); status.textContent = t("Recherche TMDB en cours…");
    const active = () => coverCurrent(x) && x.tab === "tmdb" && x.generation === generation;
    let result;
    try { result = await invoke({ action: "collection_artwork_candidates", kind: x.kind, id: x.view.id, query }); }
    catch (error) { if (active()) status.textContent = error.status === 404 || error.status === 405 ? t("Cette recherche nécessite une mise à jour du serveur.") : error.message || t("Recherche TMDB indisponible."); return; }
    if (!active()) return;
    const queue = [];
    for (const candidate of (result.candidates || []).slice(0, 24)) {
      const key = "tmdb:" + candidate.token;
      const { card, poster } = choice(candidate.title, candidate.source, key, () => select(x, { key, type: "tmdb", candidate_token: candidate.token, title: candidate.title }));
      card.disabled = true; grid.append(card); queue.push({ candidate, card, poster });
    }
    status.textContent = queue.length ? t("Choisissez une image, puis confirmez votre choix.") : t("Aucune correspondance. Essayez un autre titre ou un thème.");
    // Bound preview downloads on the Pi; closing/changing search stops the queue.
    async function previewWorker() {
      while (active() && queue.length) {
        const { candidate, card, poster } = queue.shift();
        try {
          const image = await invoke({ action: "collection_artwork_preview", kind: x.kind, id: x.view.id, token: candidate.token });
          if (!active() || !image?.src) continue;
          const img = node("img"); img.src = image.src; img.alt = candidate.title; img.decoding = "async";
          poster.replaceChildren(img); card.disabled = false;
        } catch { if (active()) poster.textContent = t("Aperçu indisponible"); }
      }
    }
    await Promise.all([previewWorker(), previewWorker()]);
  };
  $("cover-apply").onclick = async () => {
    const x = cover; if (!coverCurrent(x) || !x.selection || x.saving) return;
    const selection = x.selection, dialog = $("collection-cover-dialog"), status = $("collection-cover-status");
    x.saving = true; ++x.generation; setBusy(dialog, true); status.textContent = t("Enregistrement de la jaquette…");
    const command = { id: x.view.id, expected_revision: x.view.revision };
    if (selection.type === "member") Object.assign(command, { action: x.kind === "saga" ? "saga_member_artwork" : "playlist_member_artwork", media_id: selection.media_id });
    else Object.assign(command, { action: "collection_tmdb_artwork", kind: x.kind, candidate_token: selection.candidate_token });
    const result = await mutate(command, x, status);
    if (cover !== x) return;
    x.saving = false; setBusy(dialog, false); updateSelection();
    if (result && cover === x) { closeDialog(dialog.id); await openCollection(x.view.id, x.kind); }
  };
  $("cover-import-file").onclick = async () => {
    const x = cover; if (!coverCurrent(x) || x.saving) return;
    await importArtwork({ kind: x.kind, id: x.view.id, expected_revision: x.view.revision }, async () => { if (coverCurrent(x)) { closeDialog("collection-cover-dialog"); await openCollection(x.view.id, x.kind); } }, recoveryTarget(x, $("collection-cover-status")));
  };
  $("collection-cover-close").onclick = () => closeDialog("collection-cover-dialog");
  $("collection-cover-dialog").addEventListener("close", () => { if (!$("collection-cover-dialog").open) { ++sequence; cover = null; for (const p of $("cover-members-grid").querySelectorAll(".poster")) retireArtwork(p); } });
  $("collection-settings-dialog").addEventListener("close", () => { editor = null; });
  $("collection-delete-confirm").addEventListener("close", () => { deletion = null; });
  window.collectionManagement = { render, addTileActions, close, openCover, openEditor, openDelete, membersLoaded() { if (cover?.tab === "members") renderMembers(); } };
})();
