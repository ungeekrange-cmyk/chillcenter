(() => {
  "use strict";
  const native = window.__TAURI__?.core?.invoke;
  const state = {
    media: [],
    settings: { thumbnail_size: 200, view: "grid", theme: "system" },
    category: "all",
    sagas: false,
    watchedFilter: "all",
    sort: "title",
    busy: new Set(),
    lastScan: null,
    providers: {},
    plan: null,
    matchedIds: null,
    autoRefreshStarted: false,
    detailsRequest: 0,
    activeDetailsId: null,
    metadataRequest: 0,
    metadataMutationIds: new Set(),
    collections: [],
    activeCollectionId: null,
    collectionKindView: null,
    collectionDraft: null,
    collectionRequest: 0,
    deleteCollectionId: null,
    unavailableOnly: false,
    removeUnavailableIds: [],
    activeSeriesName: null,
    seriesSearchRequest: 0,
    seriesReturnCategory: "serie",
    sagaEditorId: null,
    sagaEditorRequest: 0,
    sagaSearchRequest: 0,
    collectionPicker: null,
    collectionSort: "year-asc",
    playlistDrag: null,
  };
  const $ = (selector, node = document) => node.querySelector(selector);
  const $$ = (selector, node = document) => [
    ...node.querySelectorAll(selector),
  ];
  const labels = {
    all: "Toute la collection",
    film: "Films",
    serie: "Séries",
    spectacle: "Spectacles",
    documentaire: "Documentaires",
  };
  const nativeUnavailable = "Cette action nécessite l’application de bureau.";

  function errorText(err) {
    const msg = String(err?.message || err || "Une erreur est survenue.");
    return msg.replace(
      /(token|secret|password)\s*[:=]\s*[^\s,]+/gi,
      "$1 masqué",
    );
  }
  async function invoke(command, args = {}) {
    if (!native) throw new Error(nativeUnavailable);
    return native(command, args);
  }
  function toast(message, isError = false) {
    const item = document.createElement("div");
    item.className = `toast${isError ? " error" : ""}`;
    item.textContent = message;
    $("#toastRegion").append(item);
    setTimeout(() => item.remove(), 4200);
  }
  function setBusy(key, busy) {
    if (busy) state.busy.add(key);
    else state.busy.delete(key);
    $$("[data-busy]")
      .filter((el) => el.dataset.busy === key)
      .forEach((el) => {
        el.disabled = busy;
      });
  }
  function esc(value) {
    return String(value ?? "").replace(
      /[&<>'"]/g,
      (c) =>
        ({
          "&": "&amp;",
          "<": "&lt;",
          ">": "&gt;",
          "'": "&#39;",
          '"': "&quot;",
        })[c],
    );
  }
  function posterSrc(path) {
    if (!path) return "";
    if (/^(https?:|asset:|data:)/i.test(path)) return path;
    try {
      return window.__TAURI__?.core?.convertFileSrc
        ? window.__TAURI__.core.convertFileSrc(path)
        : path;
    } catch (_) {
      return "";
    }
  }
  function settingsPayload() {
    const { tmdb_token, tmdb_configured, ...settings } = state.settings;
    return settings;
  }
  let presentationTimer;
  function syncControls() {
    $("#thumbnailSize").value = state.settings.thumbnail_size || 200;
    $$(".view-toggle button").forEach((button) =>
      button.classList.toggle(
        "active",
        button.dataset.view === (state.settings.view || "grid"),
      ),
    );
    $("#watchedFilter").value = state.watchedFilter;
    const sortContext = state.collections.find(
      (c) => c.id === state.activeCollectionId,
    );
    $("#sortFilter").disabled = sortContext?.kind === "playlist";
    $("#sortFilter option[value='playlist']").hidden =
      sortContext?.kind !== "playlist";
    $("#sortFilter").value =
      sortContext?.kind === "playlist"
        ? "playlist"
        : sortContext?.kind === "saga"
          ? state.collectionSort
          : state.sort;
    $("#clearAssistantFilter").hidden = !state.matchedIds;
  }
  function persistPresentation() {
    if (!native) return;
    clearTimeout(presentationTimer);
    presentationTimer = setTimeout(async () => {
      try {
        const saved = await invoke("save_settings", {
          settings: settingsPayload(),
        });
        state.settings = { ...state.settings, ...saved };
        syncControls();
      } catch (err) {
        toast(`Préférences non enregistrées : ${errorText(err)}`, true);
      }
    }, 350);
  }
  function applyTheme() {
    document.documentElement.dataset.theme = state.settings.theme || "system";
    document.documentElement.style.setProperty(
      "--poster-size",
      `${state.settings.thumbnail_size || 200}px`,
    );
  }
  function filtered() {
    const term = $("#searchInput").value.trim().toLocaleLowerCase("fr");
    const genre = $("#genreFilter").value;
    const year = $("#yearFilter").value;
    const items = state.media.filter(
      (m) =>
        (!state.unavailableOnly || m.available === false) &&
        (!state.matchedIds || state.matchedIds.has(m.id)) &&
        (!state.category ||
          state.category === "all" ||
          m.category === state.category) &&
        (state.watchedFilter === "all" ||
          (state.watchedFilter === "seen" ? m.watched : !m.watched)) &&
        (!term ||
          [
            m.title,
            m.series,
            m.saga,
            ...(m.genres || []),
            ...(m.series_metadata?.genres || []),
          ]
            .filter(Boolean)
            .join(" ")
            .toLocaleLowerCase("fr")
            .includes(term)) &&
        (!genre ||
          m.genres?.includes(genre) ||
          m.series_metadata?.genres?.includes(genre)) &&
        (!year ||
          String(m.year) === year ||
          String(m.series_metadata?.year) === year),
    );
    return items.sort(compareMedia);
  }
  function compareMedia(a, b, sort = state.sort) {
    if (sort === "year-desc")
      return (
        (b.year || -Infinity) - (a.year || -Infinity) ||
        a.title.localeCompare(b.title, "fr")
      );
    if (sort === "year-asc")
      return (
        (a.year || Infinity) - (b.year || Infinity) ||
        a.title.localeCompare(b.title, "fr")
      );
    if (sort === "genre")
      return (
        (a.genres?.[0] || "").localeCompare(b.genres?.[0] || "", "fr") ||
        a.title.localeCompare(b.title, "fr")
      );
    return a.title.localeCompare(b.title, "fr");
  }
  function collectionItemCount(items) {
    return (
      items.filter((m) => m.category !== "serie").length +
      new Set(
        items
          .filter((m) => m.category === "serie")
          .map((m) => m.series || m.title || "Série sans titre"),
      ).size
    );
  }
  function categoryLabel(m) {
    return m.category === "serie" && m.series
      ? m.series
      : labels[m.category] || m.category;
  }
  function populateFilters() {
    const genres = [
      ...new Set(
        state.media.flatMap((m) => [
          ...(m.genres || []),
          ...(m.series_metadata?.genres || []),
        ]),
      ),
    ].sort((a, b) => a.localeCompare(b, "fr"));
    const years = [
      ...new Set(
        state.media
          .flatMap((m) => [m.year, m.series_metadata?.year])
          .filter(Boolean),
      ),
    ].sort((a, b) => b - a);
    $("#genreFilter").innerHTML =
      '<option value="">Tous les genres</option>' +
      genres.map((g) => `<option>${esc(g)}</option>`).join("");
    $("#yearFilter").innerHTML =
      '<option value="">Toutes les années</option>' +
      years.map((y) => `<option>${y}</option>`).join("");
  }
  function renderCounts() {
    const missing = state.media.filter((m) => m.available === false).length;
    $("#unavailableSection").hidden = missing === 0;
    $("#unavailableCount").textContent = missing;
    $("#unavailableButton").classList.toggle("active", state.unavailableOnly);
    const count = (type) =>
      state.media.filter((m) => m.category === type).length;
    $("#allCount").textContent = collectionItemCount(state.media);
    ["film", "serie", "spectacle", "documentaire"].forEach(
      (c) => ($(`#${c}Count`).textContent = count(c)),
    );
    $("#sagaCount").textContent = state.collections.filter(
      (collection) => collection.kind === "saga",
    ).length;
    renderCollectionNav();
  }
  function renderCollectionNav() {
    const renderKind = (kind, target) => {
      const host = $(target);
      host.replaceChildren(
        ...state.collections
          .filter((collection) => collection.kind === kind)
          .map((collection) => {
            const button = document.createElement("button");
            button.className = "nav-item collection-item";
            button.textContent = collection.name;
            button.onclick = () => {
              activateCollection(collection);
            };
            return button;
          }),
      );
    };
    renderKind("playlist", "#playlistList");
  }
  function emptyState() {
    const browsing = native
      ? "Aucun élément ne correspond à ces filtres."
      : "Ouvrez cette interface dans l’application de bureau pour indexer votre cinéfolio.";
    const action = state.settings.root
      ? '<button class="primary-button" id="emptyRefresh">Indexer le dossier vidéo</button>'
      : '<button class="primary-button" id="emptySettings">Configurer le dossier vidéo</button>';
    return `<div class="empty-state"><div><div class="empty-symbol">C</div><h2>${state.media.length ? "Aucun résultat" : "Votre cinéfolio attend ses premiers films"}</h2><p>${browsing}</p>${native ? action : ""}</div></div>`;
  }
  function card(media) {
    const node = $("#cardTemplate").content.firstElementChild.cloneNode(true);
    node.dataset.mediaId = media.id;
    const img = $("img", node);
    img.src = posterSrc(media.poster);
    img.alt = media.poster ? `Affiche de ${media.title}` : "";
    const title = $("h2", node);
    title.textContent = media.title || "Sans titre";
    $(".metadata", node).textContent =
      [
        media.year,
        ...(media.genres || []).slice(0, 2),
        media.category === "serie" && media.season != null
          ? `Saison ${media.season}${media.episode != null ? ` · E${media.episode}` : ""}`
          : null,
      ]
        .filter(Boolean)
        .join(" · ") || "Métadonnées à compléter";
    const tag = $(".tag", node);
    tag.textContent =
      media.available === false
        ? "Indisponible"
        : media.watched
          ? "Vu"
          : categoryLabel(media);
    tag.classList.toggle("unavailable", media.available === false);
    const showDetails = () => openDetails(media);
    node.addEventListener("click", showDetails);
    $(".details-button", node).addEventListener("click", (ev) => {
      ev.stopPropagation();
      showDetails();
    });
    $(".play-button", node).addEventListener("click", (ev) => {
      ev.stopPropagation();
      play(media);
    });
    node.addEventListener("keydown", (ev) => {
      if (ev.target === node && (ev.key === "Enter" || ev.key === " ")) {
        ev.preventDefault();
        showDetails();
      }
    });
    return node;
  }
  function seriesRows(items) {
    const groups = new Map();
    items.forEach((m) => {
      const key = m.series || m.title;
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(m);
    });
    return [...groups.entries()]
      .map(([name, episodes]) => {
        const seasons = new Map();
        episodes.forEach((e) => {
          const key = e.season == null ? "Épisodes" : `Saison ${e.season}`;
          if (!seasons.has(key)) seasons.set(key, []);
          seasons.get(key).push(e);
        });
        const sections = [...seasons.entries()]
          .map(
            ([season, eps]) =>
              `<div class="series-season"><p class="eyebrow">${esc(season)}</p>${eps
                .sort((a, b) => (a.episode || 0) - (b.episode || 0))
                .map(
                  (e) =>
                    `<button class="episode-row" data-id="${esc(e.id)}"><span>${e.episode != null ? `E${e.episode}` : "▶"}</span><b>${esc(e.title)}</b><small>${e.watched ? "Vu" : "Non vu"}</small></button>`,
                )
                .join("")}</div>`,
          )
          .join("");
        return `<section class="series-group"><h2>${esc(name)}</h2>${sections}</section>`;
      })
      .join("");
  }
  function groupedCatalogue(items) {
    const groups = new Map();
    for (const media of items) {
      const name = state.sagas ? media.saga : media.series || media.title;
      if (!groups.has(name)) groups.set(name, []);
      groups.get(name).push(media);
    }
    return [...groups].map(([name, videos]) => {
      const section = document.createElement("section");
      section.className = "collection-group";
      const heading = document.createElement("h2");
      heading.textContent = `${name} · ${videos.length} ${state.sagas ? "films" : "épisodes"}`;
      section.append(heading);
      const seasons = new Map();
      for (const video of videos) {
        const key = state.sagas ? "Films de la saga" : (video.season ?? -1);
        if (!seasons.has(key)) seasons.set(key, []);
        seasons.get(key).push(video);
      }
      for (const [season, episodes] of [...seasons].sort(([a], [b]) =>
        typeof a === "number" ? a - b : 0,
      )) {
        const details = document.createElement("details");
        details.open = true;
        const summary = document.createElement("summary");
        summary.textContent = state.sagas
          ? season
          : season === -1
            ? "Saison non renseignée"
            : `Saison ${season}`;
        const tiles = document.createElement("div");
        tiles.className = `group-tiles ${state.settings.view || "grid"}`;
        episodes.sort((a, b) =>
          state.sagas
            ? (a.year || 0) - (b.year || 0)
            : (a.episode || 0) - (b.episode || 0),
        );
        tiles.replaceChildren(...episodes.map(card));
        details.append(summary, tiles);
        section.append(details);
      }
      return section;
    });
  }
  function seriesCatalogue(items) {
    const groups = new Map();
    for (const episode of items) {
      const name = episode.series || episode.title || "Série sans titre";
      if (!groups.has(name)) groups.set(name, []);
      groups.get(name).push(episode);
    }
    const filtersActive = Boolean(
      $("#searchInput").value ||
      $("#genreFilter").value ||
      $("#yearFilter").value ||
      state.watchedFilter !== "all" ||
      state.matchedIds,
    );
    return [...groups]
      .filter(
        ([name]) => !state.activeSeriesName || name === state.activeSeriesName,
      )
      .sort(([a], [b]) => a.localeCompare(b, "fr"))
      .map(([name, visibleEpisodes]) => {
        const allEpisodes = state.media.filter(
          (item) =>
            item.category === "serie" &&
            (item.series || item.title || "Série sans titre") === name,
        );
        const knownSeasons = new Set(
          allEpisodes
            .map((item) => item.season)
            .filter((season) => season != null),
        );
        const overview =
          allEpisodes.find((item) => item.series_metadata)?.series_metadata
            ?.overview ??
          allEpisodes.find((item) => item.series_overview)?.series_overview ??
          "";
        const common =
          allEpisodes.find((item) => item.series_metadata)?.series_metadata ||
          allEpisodes[0];
        const series = document.createElement("details");
        series.className = "series-container";
        series.open = filtersActive;
        const summary = document.createElement("summary");
        summary.innerHTML = `<span class="series-name">${esc(name)}</span><span class="series-counts">${knownSeasons.size} saison${knownSeasons.size === 1 ? "" : "s"} · ${allEpisodes.length} épisodes · ${allEpisodes.filter((item) => !item.watched).length} non vus <small>dans votre bibliothèque</small></span>`;
        series.append(summary);
        const body = document.createElement("div");
        body.className = "series-body";
        body.append(
          ratingControl(
            allEpisodes[0],
            "series",
            "Noter toute la série",
            allEpisodes,
          ),
        );
        const synopsis = document.createElement("p");
        synopsis.className = "series-overview";
        synopsis.textContent = overview || "Aucun synopsis de série renseigné.";
        body.append(synopsis);
        const facts = document.createElement("p");
        facts.className = "hint";
        facts.textContent = `${common.year || "Année inconnue"} · ${(common.genres || []).join(", ") || "Genres non renseignés"} — Distribution : ${(common.cast || []).join(", ") || "Non renseignée"} — Réalisation : ${(common.directors || []).join(", ") || "Non renseignée"}`;
        body.append(facts);
        const edit = document.createElement("details");
        edit.className = "series-overview-editor";
        edit.innerHTML = `<summary>Modifier les métadonnées de la série</summary><p class="hint">Informations communes à toute la série. Les titres, résumés, durées et autres informations propres aux épisodes restent inchangés. Aucun fichier n’est renommé.</p><form class="editor-grid"><label>Nom de la série<input name="name" required maxlength="500" value="${esc(name)}" /></label><label>Année de début<input name="year" type="number" min="1800" max="2200" value="${esc(common.year || "")}" /></label><label>Genres<input name="genres" value="${esc((common.genres || []).join(", "))}" /></label><label>Acteurs<input name="cast" value="${esc((common.cast || []).join(", "))}" /></label><label>Réalisateurs<input name="directors" value="${esc((common.directors || []).join(", "))}" /></label><label class="wide">Synopsis de la série<textarea name="overview" aria-label="Synopsis de ${esc(name)}">${esc(overview)}</textarea></label><div class="wide form-actions"><button class="secondary-button" type="submit">Enregistrer la série</button></div></form>`;
        $("form", edit).onsubmit = (event) => saveSeriesMetadata(event, name);
        const actions = document.createElement("div");
        actions.className = "detail-actions series-actions";
        const editButton = document.createElement("button");
        editButton.type = "button";
        editButton.className = "secondary-button";
        editButton.textContent = "Modifier la série";
        editButton.onclick = () => {
          edit.open = true;
          $('input[name="name"]', edit).focus();
        };
        actions.append(editButton);
        body.append(actions);
        body.append(seriesMetadataSearch(name));
        body.append(edit);
        const seasons = new Map();
        for (const episode of visibleEpisodes) {
          const key = episode.season == null ? null : episode.season;
          if (!seasons.has(key)) seasons.set(key, []);
          seasons.get(key).push(episode);
        }
        for (const [season, episodes] of [...seasons].sort(([a], [b]) =>
          a == null ? 1 : b == null ? -1 : a - b,
        )) {
          const container = document.createElement("details");
          container.className = "season-container";
          container.open = filtersActive;
          const seasonSummary = document.createElement("summary");
          seasonSummary.textContent =
            season == null ? "Saison inconnue" : `Saison ${season}`;
          const tiles = document.createElement("div");
          tiles.className = `group-tiles ${state.settings.view || "grid"}`;
          tiles.replaceChildren(
            ...episodes
              .sort(
                (a, b) =>
                  (a.episode ?? Infinity) - (b.episode ?? Infinity) ||
                  a.title.localeCompare(b.title, "fr"),
              )
              .map(card),
          );
          container.append(
            seasonSummary,
            ratingControl(
              episodes[0],
              "season",
              "Noter toute la saison",
              allEpisodes.filter((e) => e.season === season),
            ),
            tiles,
          );
          body.append(container);
        }
        series.append(body);
        return series;
      });
  }
  function seriesCards(items) {
    const names = [
      ...new Set(items.map((m) => m.series || m.title || "Série sans titre")),
    ].sort((a, b) => a.localeCompare(b, "fr"));
    return names.map((name) => {
      const episodes = state.media.filter(
        (m) =>
          m.category === "serie" &&
          (m.series || m.title || "Série sans titre") === name,
      );
      const seasons = new Set(
        episodes.map((m) => m.season).filter((n) => n != null),
      ).size;
      const unread = episodes.filter((m) => !m.watched).length;
      const poster =
        episodes.find((m) => m.poster_custom && m.poster)?.poster ||
        episodes.find((m) => m.poster)?.poster;
      const tile = document.createElement("button");
      tile.className = "series-card";
      tile.type = "button";
      tile.dataset.series = name;
      const artwork = document.createElement("div");
      artwork.className = "series-artwork";
      if (poster) {
        const img = document.createElement("img");
        img.src = posterSrc(poster);
        img.alt = `Affiche de ${name}`;
        img.loading = "lazy";
        artwork.append(img);
      } else {
        const placeholder = document.createElement("span");
        placeholder.className = "series-poster-placeholder";
        placeholder.textContent = "▤";
        placeholder.setAttribute("aria-label", "Affiche non renseignée");
        artwork.append(placeholder);
      }
      const title = document.createElement("h2");
      title.textContent = name;
      const counts = document.createElement("p");
      counts.className = "series-card-counts";
      counts.textContent = `${seasons} saison${seasons === 1 ? "" : "s"} · ${episodes.length} épisode${episodes.length === 1 ? "" : "s"} · ${unread} non vu${unread === 1 ? "" : "s"}`;
      const scope = document.createElement("small");
      scope.textContent = "Dans votre bibliothèque";
      tile.append(artwork, title, counts, scope);
      tile.onclick = () => {
        state.seriesReturnCategory = state.category === "all" ? "all" : "serie";
        state.category = "serie";
        state.activeCollectionId = null;
        state.collectionKindView = null;
        state.unavailableOnly = false;
        state.sagas = false;
        $$(".nav-item[data-category]").forEach((b) =>
          b.classList.toggle("active", b.dataset.category === "serie"),
        );
        state.activeSeriesName = name;
        render();
      };
      return tile;
    });
  }
  async function saveSeriesMetadata(event, series) {
    event.preventDefault();
    const form = event.currentTarget;
    const members = state.media.filter(
      (m) => m.category === "serie" && (m.series || m.title) === series,
    );
    if (
      state.busy.has("series-metadata") ||
      state.busy.has("apply-plan") ||
      state.busy.has("refresh") ||
      state.busy.has("enrich-catalogue") ||
      members.some((m) => state.metadataMutationIds.has(m.id))
    ) {
      toast(
        "Une opération est en cours sur cette série. Réessayez après sa fin.",
        true,
      );
      return;
    }
    const data = new FormData(form);
    const str = (name) => String(data.get(name) || "").trim();
    const list = (name) =>
      str(name)
        .split(",")
        .map((v) => v.trim())
        .filter(Boolean);
    const patch = {
      name: str("name"),
      year: Number(str("year")) || 0,
      genres: list("genres"),
      overview: str("overview"),
      cast: list("cast"),
      directors: list("directors"),
    };
    setBusy("series-metadata", true);
    members.forEach((m) => state.metadataMutationIds.add(m.id));
    $$("input,textarea,button", form).forEach(
      (control) => (control.disabled = true),
    );
    if (members.some((m) => m.id === state.activeDetailsId))
      setMetadataControlsDisabled(true);
    try {
      const updated = await invoke("update_series", { series, patch });
      for (const result of updated || []) {
        const latest = state.media.find((m) => m.id === result.id);
        // Only reconcile shared fields; unrelated pending episode results stay intact.
        if (latest) reconcileSeriesFields(latest, result);
      }
      if (state.activeSeriesName === series)
        state.activeSeriesName = patch.name;
      populateFilters();
      render();
      toast("Métadonnées de la série enregistrées.");
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      members.forEach((m) => state.metadataMutationIds.delete(m.id));
      setBusy("series-metadata", false);
      $$("input,textarea,button", form).forEach(
        (control) => (control.disabled = false),
      );
      if (members.some((m) => m.id === state.activeDetailsId))
        setMetadataControlsDisabled(false);
    }
  }
  function reconcileSeriesFields(latest, result) {
    Object.assign(latest, {
      series: result.series,
      series_manual: result.series_manual,
      series_metadata: result.series_metadata,
      series_overview: result.series_overview,
    });
    if (Object.hasOwn(result, "tmdb_id")) latest.tmdb_id = result.tmdb_id;
    if (Object.hasOwn(result, "poster")) latest.poster = result.poster;
    if (Object.hasOwn(result, "poster_custom"))
      latest.poster_custom = result.poster_custom;
  }
  function seriesMetadataSearch(series) {
    const panel = document.createElement("section");
    panel.className = "series-metadata-search";
    const form = document.createElement("form");
    form.className = "folder-row";
    const label = document.createElement("label");
    label.textContent = "Rechercher une série sur TMDB";
    const input = document.createElement("input");
    input.name = "query";
    input.maxLength = 500;
    input.value = series;
    label.append(input);
    const search = document.createElement("button");
    search.type = "submit";
    search.className = "secondary-button";
    search.textContent = "Rechercher une série";
    form.append(label, search);
    const refresh = document.createElement("button");
    refresh.type = "button";
    refresh.className = "secondary-button";
    refresh.textContent = "Actualiser les infos de la série";
    const hint = document.createElement("p");
    hint.className = "hint";
    hint.textContent =
      "Choisir une fiche remplace le nom et les informations communes de toute la série, sans changer les données propres aux épisodes. Actualiser recharge les informations communes, en conservant le nom corrigé et vos affiches personnelles.";
    const results = document.createElement("div");
    results.className = "series-candidates candidate-list";
    panel.append(form, refresh, hint, results);
    form.onsubmit = async (event) => {
      event.preventDefault();
      if (state.busy.has("series-metadata")) return;
      if (!state.settings.tmdb_configured)
        return toast("Ajoutez votre jeton TMDB dans Réglages.", true);
      const request = ++state.seriesSearchRequest;
      const query = input.value.trim() || series;
      search.disabled = true;
      results.textContent = "Recherche de séries…";
      try {
        const candidates = await invoke("series_metadata_candidates", {
          series,
          query,
        });
        if (
          !panel.isConnected ||
          state.activeSeriesName !== series ||
          request !== state.seriesSearchRequest
        )
          return;
        results.replaceChildren();
        for (const candidate of candidates.filter((c) => c.kind === "tv")) {
          const button = document.createElement("button");
          button.type = "button";
          button.className = "candidate";
          const image = document.createElement("img");
          image.src = posterSrc(candidate.poster);
          image.alt = "";
          const text = document.createElement("span");
          text.textContent = `${candidate.title} ${candidate.year || ""} — ${candidate.overview || ""}`;
          button.append(image, text);
          button.onclick = () =>
            applySeriesProvider(
              series,
              "apply_series_metadata",
              panel,
              candidate.id,
            );
          results.append(button);
        }
        if (!results.children.length)
          results.textContent = "Aucune série trouvée. Essayez un autre nom.";
      } catch (error) {
        if (panel.isConnected && request === state.seriesSearchRequest)
          results.textContent = errorText(error);
      } finally {
        if (panel.isConnected && request === state.seriesSearchRequest)
          search.disabled = false;
      }
    };
    refresh.onclick = () =>
      applySeriesProvider(series, "refresh_series_metadata", panel);
    return panel;
  }
  async function applySeriesProvider(series, command, panel, tmdbId) {
    if (!panel.isConnected || state.activeSeriesName !== series) return;
    if (!state.settings.tmdb_configured)
      return toast("Ajoutez votre jeton TMDB dans Réglages.", true);
    const members = state.media.filter(
      (m) => m.category === "serie" && (m.series || m.title) === series,
    );
    if (
      state.busy.has("series-metadata") ||
      state.busy.has("apply-plan") ||
      state.busy.has("refresh") ||
      state.busy.has("enrich-catalogue") ||
      members.some((m) => state.metadataMutationIds.has(m.id))
    )
      return toast(
        "Attendez la fin de l’opération en cours sur cette série.",
        true,
      );
    state.seriesSearchRequest += 1;
    setBusy("series-metadata", true);
    members.forEach((m) => state.metadataMutationIds.add(m.id));
    const seriesNode = panel.closest(".series-container");
    $$("input,textarea,button", seriesNode).forEach((c) => (c.disabled = true));
    if (members.some((m) => m.id === state.activeDetailsId))
      setMetadataControlsDisabled(true);
    try {
      const updated = await invoke(command, {
        series,
        ...(tmdbId == null ? {} : { tmdbId }),
      });
      for (const result of updated) {
        const latest = state.media.find((m) => m.id === result.id);
        if (latest) reconcileSeriesFields(latest, result);
      }
      if (state.activeSeriesName === series)
        state.activeSeriesName = updated[0]?.series || series;
      const active = state.media.find((m) => m.id === state.activeDetailsId);
      if (
        active &&
        members.some((m) => m.id === active.id) &&
        $("#detailsDialog")?.open
      )
        openDetails(active);
      populateFilters();
      render();
      toast(
        "Informations de la série mises à jour. Les données des épisodes sont conservées.",
      );
    } catch (error) {
      toast(errorText(error), true);
    } finally {
      members.forEach((m) => state.metadataMutationIds.delete(m.id));
      setBusy("series-metadata", false);
      if (seriesNode.isConnected)
        $$("input,textarea,button", seriesNode).forEach(
          (c) => (c.disabled = false),
        );
      if (members.some((m) => m.id === state.activeDetailsId))
        setMetadataControlsDisabled(false);
    }
  }
  function ratingControl(
    media,
    scope = "video",
    label = "Note",
    members = [media],
  ) {
    const node = document.createElement("div");
    node.className = "rating-control";
    node.dataset.scope = scope;
    const uniform = members.every((m) => m.rating === members[0].rating);
    const value = uniform ? members[0].rating || 0 : 0;
    node.innerHTML = `<span>${esc(label)}${uniform ? "" : " (notes différentes)"}</span>${[1, 2, 3, 4, 5].map((n) => `<button type="button" data-rating="${n}" aria-label="${n} étoile${n > 1 ? "s" : ""}" aria-pressed="${n === value}">${n <= value ? "★" : "☆"}</button>`).join("")}<button type="button" data-rating="0">Sans note</button>`;
    $$("[data-rating]", node).forEach(
      (button) =>
        (button.onclick = (event) => {
          event.stopPropagation();
          setRating(media, Number(button.dataset.rating), scope);
        }),
    );
    return node;
  }
  function updateFicheExtras() {
    if (!$("#detailsDialog")?.open || !state.activeDetailsId) return;
    const media = state.media.find((m) => m.id === state.activeDetailsId);
    if (!media) return;
    $("#detailsContent .rating-control")?.replaceWith(ratingControl(media));
    $("#detailsContent .membership-panel")?.replaceWith(membershipPanel(media));
  }
  async function setRating(media, rating, scope = "video") {
    const members = state.media.filter((m) =>
      scope === "video"
        ? m.id === media.id
        : m.category === "serie" &&
          (m.series || m.title) === (media.series || media.title) &&
          (scope === "series" || m.season === media.season),
    );
    if (
      state.busy.has("rating") ||
      members.some((m) => state.metadataMutationIds.has(m.id))
    )
      return;
    setBusy("rating", true);
    members.forEach((m) => state.metadataMutationIds.add(m.id));
    if (members.some((m) => m.id === state.activeDetailsId))
      setMetadataControlsDisabled(true);
    try {
      const updated = await invoke("set_rating", {
        id: media.id,
        rating,
        scope,
      });
      for (const member of members) {
        const result = updated.find((m) => m.id === member.id);
        if (result) {
          const latest = state.media.find((m) => m.id === member.id);
          if (latest) latest.rating = result.rating;
        }
      }
      render();
      updateFicheExtras();
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      members.forEach((m) => state.metadataMutationIds.delete(m.id));
      setBusy("rating", false);
      if (members.some((m) => m.id === state.activeDetailsId))
        setMetadataControlsDisabled(false);
    }
  }
  function openCollectionDialog(kind, mediaId) {
    state.collectionDraft = { kind, mediaId };
    $("#collectionDialogTitle").textContent =
      kind === "saga" ? "Créer une saga" : "Créer une liste de lecture";
    $("#collectionName").value = "";
    $("#collectionDialog").showModal();
  }
  async function createCollection(kind, name, mediaId) {
    const trimmed = String(name || "").trim();
    if (!trimmed || state.busy.has("collections")) return;
    setBusy("collections", true);
    state.collectionRequest += 1;
    try {
      state.collections = await invoke("create_collection", {
        name: trimmed,
        kind,
      });
      const created = state.collections.find(
        (collection) => collection.name === trimmed && collection.kind === kind,
      );
      if (created && mediaId)
        state.collections = await invoke("set_collection_member", {
          collectionId: created.id,
          mediaId,
          add: true,
        });
      render();
      updateFicheExtras();
      $("#collectionDialog").close();
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      setBusy("collections", false);
    }
  }
  async function setCollectionMember(collectionId, mediaId, add) {
    if (state.busy.has("collections")) return;
    setBusy("collections", true);
    state.collectionRequest += 1;
    try {
      state.collections = await invoke("set_collection_member", {
        collectionId,
        mediaId,
        add,
      });
      render();
      updateFicheExtras();
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      setBusy("collections", false);
    }
  }
  function activateCollection(collection) {
    state.activeCollectionId = collection.id;
    state.unavailableOnly = false;
    state.collectionKindView = collection.kind;
    state.category = "all";
    state.matchedIds = null;
    state.sagas = false;
    state.collectionSort = "year-asc";
    state.playlistDrag = null;
    render();
  }
  function pickerCandidates(collection) {
    return state.media.filter(
      (m) =>
        !collection.media_ids.includes(m.id) &&
        (collection.kind !== "saga" || m.category === "film"),
    );
  }
  function pickerOptions(selector, values, emptyLabel) {
    const empty = document.createElement("option");
    empty.value = "";
    empty.textContent = emptyLabel;
    $(selector).replaceChildren(
      empty,
      ...values.map((value) => {
        const option = document.createElement("option");
        option.value = String(value);
        option.textContent = labels[value] || String(value);
        return option;
      }),
    );
  }
  function openCollectionPicker(collection) {
    if (state.busy.has("collections")) return;
    state.collectionPicker = {
      id: collection.id,
      selected: new Set(),
      page: 0,
    };
    const candidates = pickerCandidates(collection);
    $("#collectionPickerTitle").textContent = `Ajouter à ${collection.name}`;
    $("#collectionPickerHint").textContent =
      collection.kind === "saga"
        ? "Sélectionnez un ou plusieurs films, puis validez. Les films déjà présents sont masqués."
        : "Sélectionnez une ou plusieurs vidéos, puis validez. Elles seront ajoutées dans l’ordre de votre sélection ; vous pourrez ensuite les déplacer dans la liste.";
    $("#collectionPickerSearch").value = "";
    pickerOptions(
      "#collectionPickerCategory",
      collection.kind === "saga"
        ? ["film"]
        : ["film", "serie", "spectacle", "documentaire"],
      "Tous les types",
    );
    pickerOptions(
      "#collectionPickerGenre",
      [
        ...new Set(
          candidates.flatMap((m) => [
            ...(m.genres || []),
            ...(m.series_metadata?.genres || []),
          ]),
        ),
      ].sort((a, b) => a.localeCompare(b, "fr")),
      "Tous les genres",
    );
    pickerOptions(
      "#collectionPickerYear",
      [
        ...new Set(
          candidates
            .flatMap((m) => [m.year, m.series_metadata?.year])
            .filter(Boolean),
        ),
      ].sort((a, b) => b - a),
      "Toutes les années",
    );
    $("#collectionPickerWatched").value = "";
    renderCollectionPicker();
    $("#collectionPickerDialog").showModal();
    $("#collectionPickerSearch").focus();
  }
  function renderCollectionPicker() {
    const context = state.collectionPicker;
    const collection = state.collections.find((c) => c.id === context?.id);
    if (!context || !collection) return;
    const term = $("#collectionPickerSearch")
      .value.trim()
      .toLocaleLowerCase("fr");
    const genre = $("#collectionPickerGenre").value;
    const year = $("#collectionPickerYear").value;
    const category = $("#collectionPickerCategory").value;
    const watched = $("#collectionPickerWatched").value;
    const candidates = pickerCandidates(collection)
      .filter(
        (m) =>
          (!term ||
            [
              m.title,
              m.series,
              ...(m.genres || []),
              ...(m.series_metadata?.genres || []),
            ]
              .filter(Boolean)
              .join(" ")
              .toLocaleLowerCase("fr")
              .includes(term)) &&
          (!genre ||
            m.genres?.includes(genre) ||
            m.series_metadata?.genres?.includes(genre)) &&
          (!year ||
            String(m.year) === year ||
            String(m.series_metadata?.year) === year) &&
          (!category || m.category === category) &&
          (!watched || (watched === "seen" ? m.watched : !m.watched)),
      )
      .sort((a, b) => compareMedia(a, b, "title"));
    const pages = Math.max(1, Math.ceil(candidates.length / 48));
    context.page = Math.min(context.page, pages - 1);
    const busy = state.busy.has("collections");
    $("#collectionPickerCards").replaceChildren(
      ...candidates
        .slice(context.page * 48, (context.page + 1) * 48)
        .map((m) => {
          const button = document.createElement("button");
          button.type = "button";
          button.className = "collection-picker-card";
          button.dataset.mediaId = m.id;
          button.setAttribute(
            "aria-pressed",
            String(context.selected.has(m.id)),
          );
          button.disabled = busy;
          const artwork = document.createElement("div");
          artwork.className = "picker-artwork";
          if (m.poster) {
            const image = document.createElement("img");
            image.src = posterSrc(m.poster);
            image.alt = "";
            image.loading = "lazy";
            artwork.append(image);
          } else {
            const empty = document.createElement("span");
            empty.textContent = "▤";
            artwork.append(empty);
          }
          const check = document.createElement("span");
          check.className = "picker-check";
          check.textContent = context.selected.has(m.id) ? "✓" : "＋";
          artwork.append(check);
          const title = document.createElement("strong");
          title.textContent = m.title;
          title.title = m.title;
          const info = document.createElement("small");
          info.textContent = [
            m.year,
            labels[m.category],
            m.category === "serie" &&
              `${m.series || ""} · S${m.season ?? "?"} E${m.episode ?? "?"}`,
            ...(m.genres || []).slice(0, 2),
            m.available === false && "Indisponible",
          ]
            .filter(Boolean)
            .join(" · ");
          button.append(artwork, title, info);
          button.onclick = () => {
            if (
              state.busy.has("collections") ||
              state.collectionPicker !== context
            )
              return;
            if (context.selected.has(m.id)) context.selected.delete(m.id);
            else if (context.selected.size < 1000) context.selected.add(m.id);
            else
              toast(
                "1 000 vidéos maximum par ajout. Validez cette sélection avant de continuer.",
                true,
              );
            renderCollectionPicker();
          };
          return button;
        }),
    );
    if (!candidates.length)
      $("#collectionPickerCards").textContent =
        "Aucune vidéo à ajouter ne correspond aux filtres.";
    $("#collectionPickerCount").textContent =
      `${candidates.length} résultat(s) · ${context.selected.size} sélectionnée(s), y compris celles masquées par les filtres`;
    $("#collectionPickerPage").textContent =
      `Page ${context.page + 1} / ${pages}`;
    $("#collectionPickerPrevious").disabled = busy || context.page === 0;
    $("#collectionPickerNext").disabled = busy || context.page + 1 === pages;
    $("#collectionPickerConfirm").disabled = busy || !context.selected.size;
    $("#collectionPickerClear").disabled = busy || !context.selected.size;
  }
  async function addPickedVideos() {
    const context = state.collectionPicker;
    if (
      !context ||
      !$("#collectionPickerDialog").open ||
      !context.selected.size ||
      state.busy.has("collections")
    )
      return;
    const mediaIds = [...context.selected];
    setBusy("collections", true);
    state.collectionRequest += 1;
    renderCollectionPicker();
    try {
      state.collections = await invoke("add_collection_members", {
        collectionId: context.id,
        mediaIds,
      });
      render();
      updateFicheExtras();
      if (
        state.collectionPicker === context &&
        $("#collectionPickerDialog").open
      )
        $("#collectionPickerDialog").close();
      toast("Sélection ajoutée. Les autres associations sont conservées.");
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      setBusy("collections", false);
      if (
        state.collectionPicker === context &&
        $("#collectionPickerDialog").open
      )
        renderCollectionPicker();
    }
  }
  async function movePlaylist(collectionId, sourceId, targetId, expectedOrder) {
    if (state.busy.has("collections") || sourceId === targetId) return;
    const order = [...expectedOrder];
    const from = order.indexOf(sourceId);
    const to = order.indexOf(targetId);
    if (from < 0 || to < 0) return;
    order.splice(from, 1);
    order.splice(to, 0, sourceId);
    setBusy("collections", true);
    state.collectionRequest += 1;
    try {
      state.collections = await invoke("reorder_playlist", {
        collectionId,
        expectedOrder,
        mediaIds: order,
      });
      render();
      updateFicheExtras();
      toast("Ordre de la liste enregistré.");
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      setBusy("collections", false);
    }
  }
  function playlistControls(collection, media, node, actions) {
    const index = collection.media_ids.indexOf(media.id);
    const number = document.createElement("span");
    number.className = "playlist-position";
    number.textContent = String(index + 1);
    $(".poster", node).append(number);
    const grip = document.createElement("button");
    grip.type = "button";
    grip.className = "playlist-drag-handle";
    grip.textContent = "⠿";
    grip.title = "Glisser pour réorganiser";
    grip.setAttribute(
      "aria-label",
      `Déplacer ${media.title} par glisser-déposer`,
    );
    grip.onclick = (event) => event.stopPropagation();
    actions.append(grip);
    for (const offset of [-1, 1]) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "playlist-move";
      button.textContent = offset < 0 ? "↑" : "↓";
      button.setAttribute(
        "aria-label",
        offset < 0 ? "Déplacer vers le haut" : "Déplacer vers le bas",
      );
      const target = collection.media_ids[index + offset];
      button.disabled = !target || state.busy.has("collections");
      if (target) button.dataset.busy = "collections";
      button.onclick = (event) => {
        event.stopPropagation();
        if (target)
          movePlaylist(collection.id, media.id, target, [
            ...collection.media_ids,
          ]);
      };
      actions.append(button);
    }
    node.draggable = true;
    $("img", node).draggable = false;
    node.ondragstart = (event) => {
      if (
        state.busy.has("collections") ||
        (event.target.closest("button") &&
          !event.target.closest(".playlist-drag-handle"))
      ) {
        event.preventDefault();
        return;
      }
      state.playlistDrag = {
        collectionId: collection.id,
        mediaId: media.id,
        order: [...collection.media_ids],
      };
      if (event.dataTransfer) {
        event.dataTransfer.effectAllowed = "move";
        event.dataTransfer.setData(
          "application/x-cinefolio-playlist",
          "reorder",
        );
      }
      node.classList.add("playlist-dragging");
    };
    node.ondragover = (event) => {
      if (
        state.playlistDrag?.collectionId !== collection.id ||
        state.busy.has("collections")
      )
        return;
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
      node.classList.add("playlist-drop-target");
    };
    node.ondragleave = () => node.classList.remove("playlist-drop-target");
    node.ondrop = (event) => {
      event.preventDefault();
      event.stopPropagation();
      const dragged = state.playlistDrag;
      state.playlistDrag = null;
      node.classList.remove("playlist-drop-target");
      if (dragged?.collectionId === collection.id)
        movePlaylist(collection.id, dragged.mediaId, media.id, dragged.order);
    };
    node.ondragend = () => {
      state.playlistDrag = null;
      $$(".playlist-dragging, .playlist-drop-target").forEach((el) =>
        el.classList.remove("playlist-dragging", "playlist-drop-target"),
      );
    };
  }
  async function reloadCollections() {
    if (state.busy.has("collections")) return;
    const request = ++state.collectionRequest;
    try {
      const collections = await invoke("get_collections");
      if (request !== state.collectionRequest) return;
      state.collections = collections;
      render();
      updateFicheExtras();
    } catch (err) {
      toast(`Collections conservées : ${errorText(err)}`, true);
    }
  }
  function confirmCollectionDelete(collection) {
    state.deleteCollectionId = collection.id;
    $("#deleteCollectionName").textContent = collection.name;
    $("#deleteCollectionDialog").showModal();
  }
  function sagaPoster(collection) {
    return (
      collection.poster ||
      state.media.find((m) => collection.media_ids.includes(m.id) && m.poster)
        ?.poster
    );
  }
  function artworkChoice(poster, title, subtitle, choose) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "artwork-choice";
    button.dataset.busy = "collections";
    button.disabled = state.busy.has("collections");
    const image = document.createElement("img");
    image.src = posterSrc(poster);
    image.alt = "";
    image.loading = "lazy";
    const label = document.createElement("span");
    label.textContent = title;
    const hint = document.createElement("small");
    hint.textContent = subtitle;
    button.append(image, label, hint);
    button.onclick = choose;
    return button;
  }
  function renderSagaImageEditor(collection) {
    const poster = sagaPoster(collection);
    $("#sagaImagePreview").src = posterSrc(poster);
    $("#sagaImagePreview").hidden = !poster;
    $("#sagaImageStatus").textContent = collection.poster
      ? "Image choisie pour cette saga. Les images des films ne changent pas."
      : "Image automatique : première image disponible des films, ou aucune image.";
    $("#sagaImageReset").disabled =
      state.busy.has("collections") || !collection.poster;
    const members = state.media.filter(
      (m) =>
        collection.media_ids.includes(m.id) &&
        m.category === "film" &&
        m.poster,
    );
    $("#sagaMemberImages").replaceChildren(
      ...members.map((m) => {
        const button = artworkChoice(m.poster, m.title, "Image du film", () =>
          mutateSaga("use_saga_member_poster", { mediaId: m.id }),
        );
        button.dataset.mediaId = m.id;
        button.setAttribute(
          "aria-pressed",
          String(collection.poster === m.poster),
        );
        return button;
      }),
    );
    if (!members.length) {
      const empty = document.createElement("p");
      empty.className = "hint";
      empty.textContent =
        "Aucun film de cette saga n’a encore d’image. Vous pouvez en importer ou en rechercher une.";
      $("#sagaMemberImages").append(empty);
    }
  }
  function openSagaEditor(collection) {
    if (state.busy.has("collections")) return;
    state.sagaEditorId = collection.id;
    state.sagaEditorRequest += 1;
    state.sagaSearchRequest += 1;
    $("#sagaName").value = collection.name;
    $("#sagaImageQuery").value = collection.name;
    $("#sagaImageResults").replaceChildren();
    $("#sagaImageSearchButton").disabled = false;
    renderSagaImageEditor(collection);
    $("#sagaEditorDialog").showModal();
    $("#sagaName").focus();
  }
  async function mutateSaga(command, args = {}) {
    const id = state.sagaEditorId;
    const editorRequest = state.sagaEditorRequest;
    if (!id || !$("#sagaEditorDialog").open || state.busy.has("collections"))
      return;
    if (command === "apply_saga_artwork" && !state.settings.tmdb_configured)
      return toast(
        "Ajoutez votre jeton TMDB dans les réglages pour rechercher une image.",
        true,
      );
    setBusy("collections", true);
    state.collectionRequest += 1;
    $("#sagaName").disabled = true;
    try {
      const collections = await invoke(command, { collectionId: id, ...args });
      if (collections) {
        state.collections = collections;
        render();
        updateFicheExtras();
        if (
          state.sagaEditorId === id &&
          state.sagaEditorRequest === editorRequest &&
          $("#sagaEditorDialog").open
        ) {
          const latest = state.collections.find((c) => c.id === id);
          if (latest) {
            if (command === "rename_saga") $("#sagaName").value = latest.name;
            renderSagaImageEditor(latest);
          }
        }
        toast(
          command === "rename_saga"
            ? "Saga renommée. Les films sont conservés."
            : "Image de la saga enregistrée.",
        );
      }
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      setBusy("collections", false);
      $("#sagaName").disabled = false;
      const latest = state.collections.find((c) => c.id === id);
      if (
        latest &&
        state.sagaEditorId === id &&
        state.sagaEditorRequest === editorRequest &&
        $("#sagaEditorDialog").open
      )
        renderSagaImageEditor(latest);
    }
  }
  async function searchSagaArtwork() {
    const id = state.sagaEditorId;
    const editorRequest = state.sagaEditorRequest;
    if (!id || !$("#sagaEditorDialog").open || state.busy.has("collections"))
      return;
    if (!state.settings.tmdb_configured)
      return toast(
        "Ajoutez votre jeton TMDB dans les réglages pour rechercher une image.",
        true,
      );
    const query = $("#sagaImageQuery").value.trim();
    if (!query) return;
    const request = ++state.sagaSearchRequest;
    const current = () =>
      state.sagaEditorId === id &&
      state.sagaEditorRequest === editorRequest &&
      state.sagaSearchRequest === request &&
      $("#sagaEditorDialog").open;
    const host = $("#sagaImageResults");
    host.textContent = "Recherche d’images…";
    $("#sagaImageSearchButton").disabled = true;
    try {
      const candidates = await invoke("saga_artwork_candidates", {
        collectionId: id,
        query,
      });
      if (!current()) return;
      host.replaceChildren(
        ...candidates.map((c) =>
          artworkChoice(c.poster, c.title, c.source, () =>
            mutateSaga("apply_saga_artwork", { posterPath: c.poster_path }),
          ),
        ),
      );
      if (!candidates.length)
        host.textContent =
          "Aucune image trouvée. Essayez un autre thème ou importez une image.";
    } catch (err) {
      if (current()) host.textContent = errorText(err);
    } finally {
      if (current()) $("#sagaImageSearchButton").disabled = false;
    }
  }
  async function deleteCollection() {
    if (state.busy.has("collections") || !state.deleteCollectionId) return;
    setBusy("collections", true);
    state.collectionRequest += 1;
    const id = state.deleteCollectionId;
    try {
      state.collections = await invoke("delete_collection", {
        collectionId: id,
      });
      if (state.activeCollectionId === id) state.activeCollectionId = null;
      render();
      updateFicheExtras();
      $("#deleteCollectionDialog").close();
      toast("Collection supprimée. Les vidéos sont conservées.");
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      setBusy("collections", false);
    }
  }
  function membershipPanel(media) {
    const panel = document.createElement("details");
    panel.className = "membership-panel";
    panel.open = true;
    const choices = state.collections.filter(
      (c) => c.kind === "playlist" || media.category === "film",
    );
    panel.innerHTML = `<summary>Sagas et listes de lecture</summary><div class="membership-list">${choices.map((c) => `<button type="button" data-collection="${esc(c.id)}" data-member="${c.media_ids.includes(media.id) ? "remove" : "add"}">${c.media_ids.includes(media.id) ? "Retirer de" : "Ajouter à"} ${esc(c.name)} (${c.kind === "saga" ? "saga" : "liste"})</button>`).join("")}</div>${media.category === "film" ? '<button type="button" id="createSagaFromFiche">Créer une saga et y ajouter ce film</button>' : ""}<button type="button" id="createPlaylistFromFiche">Créer une liste et y ajouter cette vidéo</button>`;
    $$("[data-collection]", panel).forEach(
      (b) =>
        (b.onclick = () =>
          setCollectionMember(
            b.dataset.collection,
            media.id,
            b.dataset.member === "add",
          )),
    );
    $("#createSagaFromFiche", panel)?.addEventListener("click", () =>
      openCollectionDialog("saga", media.id),
    );
    $("#createPlaylistFromFiche", panel).onclick = () =>
      openCollectionDialog("playlist", media.id);
    return panel;
  }
  function render() {
    applyTheme();
    syncControls();
    renderCounts();
    const items = filtered();
    $("#pageTitle").textContent = state.matchedIds
      ? "Résultats de l’IA"
      : state.sagas
        ? "Sagas"
        : labels[state.category] || "Collection";
    $("#sectionEyebrow").textContent =
      `${items.length} VIDÉO${items.length === 1 ? "" : "S"}`;
    const catalogue = $("#catalogue");
    if (state.unavailableOnly) {
      $("#pageTitle").textContent = "Vidéos indisponibles";
      catalogue.className = "catalogue unavailable-list";
      const header = document.createElement("div");
      header.className = "unavailable-heading";
      header.innerHTML =
        '<p>Références absentes du dernier scan réussi. Supprimer retire seulement leur fiche du catalogue local.</p><button class="secondary-button" id="removeAllUnavailable">Supprimer toutes les références indisponibles</button>';
      const allMissing = state.media.filter((m) => m.available === false);
      $("#removeAllUnavailable", header).disabled =
        !allMissing.length || state.busy.has("remove-unavailable");
      $("#removeAllUnavailable", header).onclick = () =>
        confirmRemoveUnavailable(allMissing);
      const rows = items.map((media) => {
        const row = document.createElement("article");
        row.className = "unavailable-row";
        const description = document.createElement("div");
        const title = document.createElement("button");
        title.className = "unavailable-title";
        title.textContent = media.title;
        title.onclick = () => openDetails(media);
        const path = document.createElement("p");
        path.className = "hint";
        path.textContent = `${categoryLabel(media)} · ${media.path}`;
        description.append(title, path);
        const remove = document.createElement("button");
        remove.className = "secondary-button unavailable-remove";
        remove.textContent = "Supprimer du catalogue";
        remove.disabled = state.busy.has("remove-unavailable");
        remove.onclick = () => confirmRemoveUnavailable([media]);
        row.append(description, remove);
        return row;
      });
      if (!rows.length) {
        const empty = document.createElement("p");
        empty.textContent = allMissing.length
          ? "Aucune vidéo indisponible ne correspond aux filtres."
          : "Aucune vidéo indisponible. Le catalogue est à jour.";
        rows.push(empty);
      }
      catalogue.replaceChildren(header, ...rows);
      return;
    }
    const activeCollection = state.collections.find(
      (collection) => collection.id === state.activeCollectionId,
    );
    if (activeCollection) {
      const members = activeCollection.media_ids
        .map((id) => state.media.find((media) => media.id === id))
        .filter((m) => m && items.some((item) => item.id === m.id));
      if (activeCollection.kind === "saga")
        members.sort((a, b) => compareMedia(a, b, state.collectionSort));
      $("#pageTitle").textContent = activeCollection.name;
      catalogue.className = `catalogue ${state.settings.view || "grid"}`;
      const heading = document.createElement("div");
      heading.className = "collection-heading";
      heading.innerHTML = `<button class="secondary-button" id="collectionBack">← Collections</button><h2>${esc(activeCollection.name)}</h2><p>${members.length} vidéo${members.length === 1 ? "" : "s"}</p>`;
      catalogue.replaceChildren(heading, ...members.map(card));
      const addButton = document.createElement("button");
      addButton.type = "button";
      addButton.id = "collectionAddVideos";
      addButton.className = "primary-button";
      addButton.dataset.busy = "collections";
      addButton.textContent = "＋ Ajouter des vidéos";
      addButton.disabled = state.busy.has("collections");
      addButton.onclick = () => openCollectionPicker(activeCollection);
      heading.append(addButton);
      if (activeCollection.kind === "playlist") {
        const hint = document.createElement("p");
        hint.className = "hint";
        hint.textContent =
          "Ordre d’ajout ou ordre personnalisé. Glissez une carte à la position souhaitée, ou utilisez ↑ et ↓. Les recherches masquent des vidéos sans changer leur ordre.";
        heading.append(hint);
      }
      const deleteButton = document.createElement("button");
      deleteButton.id = "deleteActiveCollection";
      deleteButton.textContent = "Supprimer cette collection";
      deleteButton.onclick = () => confirmCollectionDelete(activeCollection);
      heading.append(deleteButton);
      if (activeCollection.kind === "saga") {
        const edit = document.createElement("button");
        edit.id = "editActiveSaga";
        edit.className = "secondary-button";
        edit.textContent = "Modifier la saga";
        edit.onclick = () => openSagaEditor(activeCollection);
        heading.append(edit);
      }
      $$(".media-card", catalogue).forEach((node, index) => {
        const actions = document.createElement("div");
        actions.className = "collection-member-actions";
        if (activeCollection.kind === "playlist")
          playlistControls(activeCollection, members[index], node, actions);
        const remove = document.createElement("button");
        remove.type = "button";
        remove.className = "collection-remove";
        remove.textContent = "Retirer";
        remove.setAttribute("aria-label", "Retirer de cette collection");
        remove.dataset.busy = "collections";
        remove.disabled = state.busy.has("collections");
        remove.onclick = (event) => {
          event.stopPropagation();
          setCollectionMember(activeCollection.id, members[index].id, false);
        };
        actions.append(remove);
        node.append(actions);
      });
      $("#collectionBack").onclick = () => {
        state.activeCollectionId = null;
        state.collectionKindView = activeCollection.kind;
        render();
      };
      return;
    }
    if (state.collectionKindView || state.sagas) {
      const kind = state.collectionKindView || "saga";
      $("#pageTitle").textContent =
        kind === "saga" ? "Sagas" : "Listes de lecture";
      catalogue.className = "catalogue collection-cards";
      const create = document.createElement("button");
      create.className = "primary-button collection-create-main";
      create.textContent =
        kind === "saga" ? "＋ Créer une saga" : "＋ Créer une liste de lecture";
      create.onclick = () => openCollectionDialog(kind);
      const term = $("#searchInput").value.trim().toLocaleLowerCase("fr");
      const collections = state.collections.filter(
        (c) =>
          c.kind === kind &&
          (!term ||
            c.name.toLocaleLowerCase("fr").includes(term) ||
            c.media_ids.some((id) => items.some((m) => m.id === id))),
      );
      catalogue.replaceChildren(
        create,
        ...collections.map((c) => {
          const tile = document.createElement("button");
          tile.className = "collection-card";
          tile.dataset.collection = c.id;
          const poster = sagaPoster(c);
          const artwork = document.createElement("div");
          artwork.className = "collection-artwork";
          if (poster) {
            const img = document.createElement("img");
            img.src = posterSrc(poster);
            img.alt = "";
            artwork.append(img);
          } else {
            const placeholder = document.createElement("span");
            placeholder.className = "collection-placeholder";
            placeholder.textContent = c.kind === "saga" ? "◈" : "☷";
            placeholder.setAttribute("aria-label", "Image non renseignée");
            artwork.append(placeholder);
          }
          tile.append(artwork);
          const title = document.createElement("span");
          title.textContent = `${c.name} · ${c.media_ids.length} vidéo${c.media_ids.length === 1 ? "" : "s"}`;
          tile.append(title);
          tile.onclick = () => {
            activateCollection(c);
          };
          const wrapper = document.createElement("article");
          wrapper.className = "collection-tile";
          const remove = document.createElement("button");
          remove.className = "collection-delete";
          remove.textContent = "Supprimer";
          remove.onclick = () => confirmCollectionDelete(c);
          const actions = document.createElement("div");
          actions.className = "collection-tile-actions";
          if (kind === "saga") {
            const edit = document.createElement("button");
            edit.className = "collection-edit secondary-button";
            edit.textContent = "Modifier la saga";
            edit.onclick = () => openSagaEditor(c);
            actions.append(edit);
          }
          actions.append(remove);
          wrapper.append(tile, actions);
          return wrapper;
        }),
      );
      return;
    }
    catalogue.className = `catalogue ${state.settings.view || "grid"}`;
    if (state.category === "serie" && state.activeSeriesName) {
      catalogue.className = "catalogue series-catalogue";
      $("#pageTitle").textContent = state.activeSeriesName;
      const back = document.createElement("button");
      back.id = "seriesBack";
      back.className = "secondary-button";
      back.textContent =
        state.seriesReturnCategory === "all"
          ? "← Toute la collection"
          : "← Toutes les séries";
      back.onclick = () => {
        state.activeSeriesName = null;
        state.category = state.seriesReturnCategory;
        $$(".nav-item[data-category]").forEach((b) =>
          b.classList.toggle("active", b.dataset.category === state.category),
        );
        render();
      };
      const sections = seriesCatalogue(items);
      sections.forEach((s) => (s.open = true));
      catalogue.replaceChildren(back, ...sections);
      if (!sections.length) {
        const empty = document.createElement("p");
        empty.textContent =
          "Aucun épisode de cette série ne correspond aux filtres.";
        catalogue.append(empty);
      }
      return;
    }
    if (!items.length) {
      catalogue.innerHTML = emptyState();
      $("#emptyRefresh")?.addEventListener("click", refresh);
      $("#emptySettings")?.addEventListener("click", openSettings);
      return;
    }
    if (state.category === "serie") {
      catalogue.className = `catalogue series-cards ${state.settings.view || "grid"}`;
      catalogue.replaceChildren(...seriesCards(items));
      return;
    }
    if (state.category === "all") {
      const entries = items
        .filter((m) => m.category !== "serie")
        .map((m) => ({ media: m, node: card(m) }));
      for (const node of seriesCards(
        items.filter((m) => m.category === "serie"),
      )) {
        const name = node.dataset.series;
        const members = state.media.filter(
          (m) =>
            m.category === "serie" &&
            (m.series || m.title || "Série sans titre") === name,
        );
        const common =
          members.find((m) => m.series_metadata)?.series_metadata || members[0];
        entries.push({ media: { ...common, title: name }, node });
      }
      entries.sort((a, b) => compareMedia(a.media, b.media));
      catalogue.classList.add("mixed-collection");
      $("#sectionEyebrow").textContent =
        `${entries.length} ÉLÉMENT${entries.length === 1 ? "" : "S"}`;
      catalogue.replaceChildren(...entries.map((e) => e.node));
      return;
    }
    catalogue.replaceChildren(...items.map(card));
  }
  function confirmRemoveUnavailable(media) {
    if (state.busy.has("remove-unavailable") || !media.length) return;
    state.removeUnavailableIds = media.map((m) => m.id);
    $("#removeUnavailableSummary").textContent =
      media.length === 1
        ? `Retirer « ${media[0].title} » ?`
        : `Retirer les ${media.length} vidéos indisponibles, y compris celles masquées par les filtres ?`;
    $("#removeUnavailableDialog").showModal();
  }
  async function removeUnavailable() {
    const ids = [...state.removeUnavailableIds];
    if (!ids.length || state.busy.has("remove-unavailable")) return;
    if (
      ids.some((id) => state.metadataMutationIds.has(id)) ||
      state.busy.has("collections") ||
      state.busy.has("refresh") ||
      state.busy.has("enrich-catalogue")
    ) {
      return toast(
        "Attendez la fin des modifications en cours avant de supprimer ces références.",
        true,
      );
    }
    setBusy("remove-unavailable", true);
    setBusy("collections", true);
    state.collectionRequest += 1;
    $("#confirmRemoveUnavailable").disabled = true;
    ids.forEach((id) => state.metadataMutationIds.add(id));
    render();
    try {
      const deleted = new Set(await invoke("remove_unavailable", { ids }));
      state.media = state.media.filter((m) => !deleted.has(m.id));
      state.collections = state.collections.map((c) => ({
        ...c,
        media_ids: c.media_ids.filter((id) => !deleted.has(id)),
      }));
      if (deleted.has(state.activeDetailsId)) {
        state.detailsRequest += 1;
        state.activeDetailsId = null;
        $("#detailsDialog").close();
      }
      state.removeUnavailableIds = [];
      populateFilters();
      render();
      $("#removeUnavailableDialog").close();
      toast(
        `${deleted.size} référence(s) supprimée(s) du catalogue. Aucun fichier supprimé.`,
      );
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      ids.forEach((id) => state.metadataMutationIds.delete(id));
      setBusy("remove-unavailable", false);
      setBusy("collections", false);
      $("#confirmRemoveUnavailable").disabled = false;
      render();
    }
  }
  async function enrichCatalogue() {
    if (state.busy.has("series-metadata"))
      return toast("Attendez la fin des corrections de la série.", true);
    if (state.busy.has("remove-unavailable"))
      return toast("Attendez la fin de la suppression des références.", true);
    if (!state.settings.tmdb_configured) {
      openSettings();
      toast("Renseignez le jeton de lecture TMDB pour compléter les fiches.");
      return;
    }
    const button = $("#enrichCatalogueButton");
    setBusy("enrich-catalogue", true);
    button.disabled = true;
    button.textContent = "Recherche des fiches…";
    try {
      const result = await invoke("enrich_catalog");
      const data = await invoke("bootstrap");
      state.media = data.media || state.media;
      await reloadCollections();
      populateFilters();
      render();
      toast(
        `${result.updated} fiche(s) complétée(s). ${result.warnings?.length ? `${result.warnings.length} correspondance(s) à vérifier dans Détails.` : ""}`,
      );
    } catch (error) {
      toast(errorText(error), true);
    } finally {
      setBusy("enrich-catalogue", false);
      button.disabled = false;
      button.textContent = "Compléter les fiches";
    }
  }
  async function bootstrap() {
    if (!native) {
      $$("#refreshButton,#assistantButton,#settingsButton").forEach(
        (button) => {
          button.disabled = true;
          button.title = nativeUnavailable;
        },
      );
      $("#scanStatus").textContent =
        "Aperçu navigateur — fonctions natives indisponibles";
      render();
      toast("Aperçu navigateur : les actions natives sont désactivées.");
      return;
    }
    try {
      const data = await invoke("bootstrap");
      state.media = data.media || [];
      state.settings = { ...state.settings, ...(data.settings || {}) };
      state.lastScan = data.last_scan;
      state.providers = data.providers || {};
      try {
        state.collections = await invoke("get_collections");
        render();
      } catch (_) {
        state.collections = [];
      }
      $("#scanStatus").textContent = data.scan_running
        ? "Indexation en cours…"
        : state.lastScan
          ? `Mis à jour ${new Date(state.lastScan * 1000).toLocaleDateString("fr-FR")}`
          : "Prêt à cataloguer";
      populateFilters();
      render();
      if (
        state.settings.auto_refresh &&
        state.settings.root &&
        !data.scan_running &&
        !state.autoRefreshStarted
      ) {
        state.autoRefreshStarted = true;
        refresh();
      }
    } catch (err) {
      render();
      toast(errorText(err), true);
    }
  }
  async function refresh() {
    if (state.busy.has("series-metadata"))
      return toast("Attendez la fin des corrections de la série.", true);
    if (state.busy.has("remove-unavailable"))
      return toast("Attendez la fin de la suppression des références.", true);
    setBusy("refresh", true);
    $("#refreshButton").classList.add("spinning");
    $("#scanStatus").textContent = "Indexation du catalogue en cours…";
    try {
      const data = await invoke("refresh_catalog");
      state.media = data.media || state.media;
      await reloadCollections();
      state.lastScan = data.last_scan;
      populateFilters();
      render();
      const warning = data.warnings?.length
        ? ` ${data.warnings.join(" · ")}`
        : "";
      toast(`Catalogue actualisé.${warning}`);
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      setBusy("refresh", false);
      $("#refreshButton").classList.remove("spinning");
      $("#scanStatus").textContent = state.lastScan
        ? `Mis à jour ${new Date(state.lastScan * 1000).toLocaleDateString("fr-FR")}`
        : "Actualisation interrompue";
    }
  }
  async function play(media) {
    if (state.busy.has(`play-${media.id}`)) return;
    setBusy(`play-${media.id}`, true);
    try {
      const updated = await invoke("play_media", { id: media.id });
      reconcileMedia(media, updated || { watched: true });
      render();
      if (isActiveDetails(media, state.detailsRequest)) {
        const toggle = $("#watchToggle");
        toggle.textContent = "Marquer non vu";
        toggle.onclick = () => setWatched(media, false);
      }
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      setBusy(`play-${media.id}`, false);
    }
  }
  function openDetails(media) {
    if (!media) return;
    const dialog = $("#detailsDialog");
    const request = ++state.detailsRequest;
    state.metadataRequest += 1;
    state.activeDetailsId = media.id;
    const genres = media.genres?.join(", ") || "Non renseigné";
    const duration = media.runtime_minutes
      ? `${media.runtime_minutes} min`
      : "Durée inconnue";
    const cast = media.cast?.length ? media.cast.join(", ") : "Non renseignés";
    const directors = media.directors?.length
      ? media.directors.join(", ")
      : "Non renseignés";
    $("#detailsContent").innerHTML =
      `<div class="presentation-hero"><img class="detail-poster" src="${esc(posterSrc(media.poster))}" alt="Affiche de ${esc(media.title)}" /><div><p class="eyebrow">${esc(categoryLabel(media))}</p><h2>${esc(media.title)}</h2><div class="detail-meta"><span>${esc(media.year || "Année inconnue")}</span><span>${esc(genres)}</span>${media.saga ? `<span>Saga : ${esc(media.saga)}</span>` : ""}</div><p class="detail-overview">${esc(media.overview || "Aucun résumé disponible.")}</p><dl class="presentation-facts"><div><dt>Durée</dt><dd>${esc(duration)}</dd></div><div><dt>Réalisation</dt><dd>${esc(directors)}</dd></div><div><dt>Distribution</dt><dd>${esc(cast)}</dd></div></dl><div class="detail-actions"><button class="primary-button" id="detailPlay">▶ Lire</button><button class="secondary-button" id="watchToggle">${media.watched ? "Marquer non vu" : "Marquer vu"}</button><button class="secondary-button" id="enrichButton">Enrichir TMDB</button></div></div></div><section class="related-section" aria-live="polite"><p class="eyebrow">DANS VOTRE CINÉFOLIO</p><div id="detailsExtra"><p class="hint">Recherche des vidéos proches…</p></div></section><details class="editor-section"><summary>Modifier les métadonnées</summary><form id="editorForm" class="editor-grid"><label>Titre<input name="title" value="${esc(media.title)}" /></label><label>Catégorie<select name="category">${["film", "serie", "spectacle", "documentaire"].map((c) => `<option value="${c}" ${media.category === c ? "selected" : ""}>${labels[c]}</option>`).join("")}</select></label><label>Année<input name="year" type="number" value="${esc(media.year || "")}" /></label><label>Genres<input name="genres" value="${esc(genres === "Non renseigné" ? "" : genres)}" placeholder="Drame, Comédie" /></label><label>Série<input name="series" value="${esc(media.series || "")}" /></label><label>Saison / épisode<div class="folder-row"><input name="season" type="number" value="${esc(media.season ?? "")}" placeholder="Saison" /><input name="episode" type="number" value="${esc(media.episode ?? "")}" placeholder="Épisode" /></div></label><label>Saga<input name="saga" value="${esc(media.saga || "")}" placeholder="ex. Le Parrain" /></label><label class="wide">Résumé<textarea name="overview">${esc(media.overview || "")}</textarea></label><div class="wide form-actions"><button class="primary-button" type="submit">Enregistrer les métadonnées</button></div></form></details>`;
    $("#detailPlay").onclick = () => play(media);
    $("#watchToggle").onclick = () => setWatched(media, !media.watched);
    $("#enrichButton").onclick = () => enrich(media);
    $("#detailsContent .detail-actions").before(ratingControl(media));
    $("#editorForm").onsubmit = (e) => updateMedia(e, media);
    $('input[name="saga"]', $("#editorForm")).closest("label").hidden = true;
    const facts = $(".detail-meta");
    $$("span", facts)
      .filter((n) => n.textContent.startsWith("Saga :"))
      .forEach((n) => n.remove());
    const tmdb = document.createElement("span");
    tmdb.textContent = media.tmdb_id
      ? `TMDB #${media.tmdb_id}`
      : "Aucune association TMDB";
    facts.append(tmdb);
    const search = document.createElement("div");
    search.className = "metadata-search";
    search.innerHTML =
      '<input id="metadataQuery" aria-label="Rechercher une autre fiche TMDB" placeholder="Rechercher une fiche TMDB" /><button class="secondary-button" id="metadataSearchButton" type="button">Choisir une autre fiche</button>';
    $(".related-section").before(search);
    const candidateSlot = document.createElement("div");
    candidateSlot.id = "detailsCandidates";
    search.after(candidateSlot);
    $(".related-section").after(membershipPanel(media));
    $("#metadataSearchButton").onclick = () => enrich(media);
    const extraActions = $("#detailsContent .detail-actions");
    for (const [id, label] of [
      ["importPosterButton", "Choisir une affiche"],
      ["refreshMetadataButton", "Forcer l’actualisation"],
    ]) {
      const button = document.createElement("button");
      button.id = id;
      button.type = "button";
      button.className = "secondary-button";
      button.textContent = label;
      extraActions.append(button);
    }
    if (media.poster_custom && media.tmdb_id) {
      const reset = document.createElement("button");
      reset.id = "resetPosterButton";
      reset.type = "button";
      reset.className = "secondary-button";
      reset.textContent = "Restaurer l’affiche TMDB";
      extraActions.append(reset);
      reset.onclick = (event) =>
        resetPoster(media, request, event.currentTarget);
    }
    $("#importPosterButton").onclick = (event) =>
      importPoster(media, request, event.currentTarget);
    $("#refreshMetadataButton").onclick = (event) =>
      refreshMetadata(media, request, event.currentTarget);
    const forceHint = document.createElement("p");
    forceHint.className = "hint";
    forceHint.textContent =
      "L’actualisation remplace les métadonnées par TMDB, sauf votre affiche personnalisée et les modifications simultanées.";
    extraActions.after(forceHint);
    const editor = $("#editorForm");
    if (media.category === "serie")
      $("summary", editor.closest("details")).textContent =
        "Modifier les métadonnées de cet épisode";
    const relativeField = document.createElement("label");
    relativeField.className = "wide";
    relativeField.textContent = "Chemin relatif à la racine de la bibliothèque";
    const relativePath = document.createElement("input");
    relativePath.name = "relative_path";
    relativePath.readOnly = true;
    relativePath.value = media.id;
    relativeField.append(relativePath);
    editor.insertBefore(relativeField, $(".form-actions", editor));
    const location = document.createElement("div");
    const locationLabel = document.createElement("dt");
    locationLabel.textContent = "Chemin relatif à la bibliothèque";
    const locationValue = document.createElement("dd");
    locationValue.textContent = media.id;
    location.append(locationLabel, locationValue);
    $(".presentation-facts").append(location);
    addEditorField(
      editor,
      "runtime_minutes",
      "Durée (minutes)",
      media.runtime_minutes ?? "",
      "number",
    );
    addEditorField(
      editor,
      "cast",
      "Acteurs (séparés par des virgules)",
      (media.cast || []).join(", "),
    );
    addEditorField(
      editor,
      "directors",
      "Réalisateurs (séparés par des virgules)",
      (media.directors || []).join(", "),
    );
    dialog.showModal();
    setMetadataControlsDisabled(state.metadataMutationIds.has(media.id));
    similar(media, request);
  }
  function addEditorField(form, name, label, value, type = "text") {
    const field = document.createElement("label");
    field.textContent = label;
    const input = document.createElement("input");
    input.name = name;
    input.type = type;
    input.value = value;
    field.append(input);
    form.insertBefore(field, $(".wide", form));
  }
  function patchFromForm(form) {
    const d = new FormData(form);
    const str = (v) => String(d.get(v) || "").trim();
    const number = (v) => {
      const x = str(v);
      return x ? Number(x) : null;
    };
    return {
      title: str("title"),
      category: str("category"),
      year: number("year") || 0,
      runtime_minutes: number("runtime_minutes") || 0,
      genres: str("genres")
        .split(",")
        .map((x) => x.trim())
        .filter(Boolean),
      series: str("series"),
      season: number("season"),
      episode: number("episode"),
      saga: str("saga"),
      overview: str("overview"),
      cast: str("cast")
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean),
      directors: str("directors")
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean),
    };
  }
  function reconcileMedia(media, updated) {
    Object.assign(media, updated);
    const canonical = state.media.find((item) => item.id === media.id);
    if (canonical && canonical !== media) Object.assign(canonical, updated);
    if (Object.hasOwn(updated, "category") || Object.hasOwn(updated, "saga"))
      void reloadCollections();
  }
  async function updateMedia(event, media) {
    event.preventDefault();
    const button = $('button[type="submit"]', event.currentTarget);
    const request = state.detailsRequest;
    const patch = patchFromForm(event.currentTarget);
    return runMetadataMutation(media, request, async () => {
      button.disabled = true;
      try {
        const updated = await invoke("update_media", {
          id: media.id,
          patch,
        });
        reconcileMedia(media, updated);
        populateFilters();
        render();
        if (!isActiveDetails(media, request)) return;
        toast("Métadonnées enregistrées.");
        openDetails(media);
      } catch (err) {
        if (isActiveDetails(media, request)) toast(errorText(err), true);
      } finally {
        button.disabled = false;
      }
    });
  }
  async function setWatched(media, watched) {
    const request = state.detailsRequest;
    return runMetadataMutation(media, request, async () => {
      try {
        const updated = await invoke("set_watched", { id: media.id, watched });
        reconcileMedia(media, updated);
        render();
        if (!isActiveDetails(media, request)) return;
        openDetails(media);
      } catch (err) {
        if (isActiveDetails(media, request)) toast(errorText(err), true);
      }
    });
  }
  async function enrich(media) {
    if (state.metadataMutationIds.has(media.id)) return;
    if (!state.settings.tmdb_configured)
      return toast(
        "Configurez un jeton TMDB dans Réglages avant l’enrichissement.",
        true,
      );
    const request = state.detailsRequest;
    const metadataRequest = ++state.metadataRequest;
    const query = $("#metadataQuery")?.value.trim() || media.title;
    try {
      const candidates = await invoke("metadata_candidates", {
        id: media.id,
        query,
      });
      if (
        !isActiveDetails(media, request) ||
        metadataRequest !== state.metadataRequest
      )
        return;
      const extra = $("#detailsCandidates");
      extra.innerHTML = `<div class="candidate-list"><p class="eyebrow">RÉSULTATS TMDB</p>${!candidates.length ? '<p class="hint">Aucune correspondance trouvée.</p>' : candidates.map((c) => `<button class="candidate" data-tmdb="${esc(c.id)}" data-kind="${esc(c.kind)}"><img src="${esc(posterSrc(c.poster))}" alt="" /><span><b>${esc(c.title)}</b><br><small>${c.year || ""} · ${esc(c.overview || "")}</small></span></button>`).join("")}</div>`;
      $$(".candidate", extra).forEach(
        (b) =>
          (b.onclick = () =>
            runMetadataMutation(media, request, async () => {
              try {
                if (!isActiveDetails(media, request)) return;
                const updated = await invoke("apply_metadata", {
                  id: media.id,
                  tmdbId: Number(b.dataset.tmdb),
                  kind: b.dataset.kind,
                });
                reconcileMedia(media, updated);
                populateFilters();
                render();
                if (!isActiveDetails(media, request)) return;
                toast("Métadonnées TMDB appliquées.");
                openDetails(media);
              } catch (err) {
                if (isActiveDetails(media, request))
                  toast(errorText(err), true);
              }
            })),
      );
    } catch (err) {
      toast(errorText(err), true);
    }
  }
  function isActiveDetails(media, request) {
    try {
      return (
        $("#detailsDialog")?.open &&
        state.activeDetailsId === media.id &&
        state.detailsRequest === request
      );
    } catch (_) {
      return false;
    }
  }
  function setMetadataControlsDisabled(disabled) {
    $$(
      "#metadataQuery,#enrichButton,#metadataSearchButton,#refreshMetadataButton,#resetPosterButton,#importPosterButton,#watchToggle,#editorForm input,#editorForm select,#editorForm textarea,#editorForm button,[data-tmdb],[data-rating]",
    ).forEach((button) => (button.disabled = disabled));
  }
  async function runMetadataMutation(media, request, work) {
    if (state.metadataMutationIds.has(media.id)) return;
    state.metadataMutationIds.add(media.id);
    if (isActiveDetails(media, request)) setMetadataControlsDisabled(true);
    try {
      await work();
    } finally {
      state.metadataMutationIds.delete(media.id);
      if ($("#detailsDialog")?.open && state.activeDetailsId === media.id)
        setMetadataControlsDisabled(false);
    }
  }
  async function updatePresentation(media, request, command, button) {
    if (button?.disabled) return;
    if (button) button.disabled = true;
    try {
      const updated = await invoke(command, { id: media.id });
      if (!updated) return;
      reconcileMedia(media, updated);
      populateFilters();
      render();
      if (!isActiveDetails(media, request)) return;
      openDetails(media);
    } catch (err) {
      if (isActiveDetails(media, request)) toast(errorText(err), true);
    } finally {
      if (button && isActiveDetails(media, request)) button.disabled = false;
    }
  }
  function importPoster(media, request, button) {
    return runMetadataMutation(media, request, () =>
      updatePresentation(media, request, "import_poster"),
    );
  }
  async function resetPoster(media, request, button) {
    await runMetadataMutation(media, request, () =>
      updatePresentation(media, request, "reset_poster"),
    );
  }
  function refreshMetadata(media, request, button) {
    return runMetadataMutation(media, request, () =>
      updatePresentation(media, request, "refresh_metadata"),
    );
  }
  async function similar(media, request) {
    try {
      const matches = await invoke("similar_media", { id: media.id });
      const dialog = $("#detailsDialog");
      if (
        !dialog ||
        !dialog.open ||
        request !== state.detailsRequest ||
        state.activeDetailsId !== media.id
      )
        return;
      $("#detailsExtra").innerHTML =
        `<div class="similar-list">${matches.length ? matches.map((m) => `<button class="candidate" data-media="${esc(m.id)}"><span><b>${esc(m.title)}</b><br><small>${esc([m.year, ...(m.genres || [])].filter(Boolean).join(" · "))}</small></span></button>`).join("") : '<p class="hint">Aucun titre similaire dans votre catalogue.</p>'}</div>`;
      $$("[data-media]", $("#detailsExtra")).forEach(
        (b) =>
          (b.onclick = () =>
            openDetails(state.media.find((m) => m.id === b.dataset.media))),
      );
    } catch (_) {
      if (isActiveDetails(media, request))
        $("#detailsExtra").innerHTML =
          '<p class="hint">Vidéos similaires indisponibles.</p>';
    }
  }
  function openSettings() {
    const form = $("#settingsForm");
    form.elements.clear_tmdb.checked = false;
    form.elements.tmdb_token.value = "";
    Object.entries(state.settings).forEach(([key, value]) => {
      const el = form.elements[key];
      if (!el) return;
      if (el.type === "checkbox") el.checked = Boolean(value);
      else if (el.type === "radio") {
        const radio = form.querySelector(`[name="${key}"][value="${value}"]`);
        if (radio) radio.checked = true;
      } else el.value = value ?? "";
    });
    $("#tokenHelp").textContent = state.settings.tmdb_configured
      ? "Un jeton TMDB est déjà configuré. Laissez vide pour le conserver."
      : "Le jeton est conservé dans le Trousseau macOS.";
    $("#settingsDialog").showModal();
  }
  async function saveSettings(event) {
    event.preventDefault();
    const form = event.currentTarget;
    const data = new FormData(form);
    const settings = {
      ...settingsPayload(),
      root: String(data.get("root") || ""),
      auto_refresh: data.get("auto_refresh") === "on",
      theme: String(data.get("theme")),
      player: String(data.get("player")),
      ai_provider: String(data.get("ai_provider")),
      codex_path: String(data.get("codex_path") || ""),
      claude_path: String(data.get("claude_path") || ""),
      thumbnail_size: state.settings.thumbnail_size,
      view: state.settings.view,
    };
    const token = String(data.get("tmdb_token") || "");
    const clearTmdb = !token && data.get("clear_tmdb") === "on";
    if (token) settings.tmdb_token = token;
    try {
      const saved = await invoke("save_settings", {
        settings,
        ...(clearTmdb ? { clearTmdb: true } : {}),
      });
      state.settings = { ...state.settings, ...saved };
      applyTheme();
      syncControls();
      $("#settingsDialog").close();
      form.elements.tmdb_token.value = "";
      form.elements.clear_tmdb.checked = false;
      toast("Réglages enregistrés.");
    } catch (err) {
      toast(errorText(err), true);
    }
  }
  async function selectFolder() {
    try {
      const root = await invoke("select_folder");
      if (root) $("#settingsForm").elements.root.value = root;
    } catch (err) {
      toast(errorText(err), true);
    }
  }
  function openAssistant() {
    const d = $("#assistantDialog");
    $("#assistantContent").innerHTML =
      `<p class="eyebrow">ORGANISATION ASSISTÉE</p><h2>Proposer un rangement</h2><p class="assistant-disclosure">Seuls les titres et métadonnées de votre catalogue seront envoyés au fournisseur sélectionné. Aucun fichier vidéo ne quitte votre Mac. Votre authentification CLI et vos quotas existants sont utilisés.</p><textarea class="assistant-request" id="assistantRequest" placeholder="Ex. Regroupe les films Star Wars en saga et complète les séries sans saison."></textarea><div class="form-actions"><button class="secondary-button" id="speechButton">⌁ Dicter (12 s)</button><button class="primary-button" id="planButton">Prévisualiser les changements</button></div><div id="planResult"></div>`;
    $("#speechButton").onclick = transcribe;
    $("#planButton").onclick = assistantPlan;
    d.showModal();
  }
  async function transcribe() {
    const b = $("#speechButton");
    b.disabled = true;
    b.textContent = "Écoute en cours…";
    try {
      const text = await invoke("transcribe");
      $("#assistantRequest").value = text;
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      b.disabled = false;
      b.textContent = "⌁ Dicter (12 s)";
    }
  }
  async function assistantPlan() {
    const request = $("#assistantRequest").value.trim();
    if (!request) return toast("Décrivez l’organisation souhaitée.", true);
    const b = $("#planButton");
    b.disabled = true;
    try {
      const plan = await invoke("assistant_plan", { request });
      state.plan = plan;
      const changes = plan.changes || [];
      const matched = new Set(plan.matched_ids || []);
      $("#planResult").innerHTML =
        `<div class="assistant-disclosure"><b>Prévisualisation</b><br>${esc(plan.message || "")}</div><div class="plan-item">${matched.size} titre(s) correspondant(s) consultable(s)</div>${
          changes
            .map((c) => {
              const fields = Object.entries(c)
                .filter(([key]) => key !== "id" && key !== "title")
                .map(
                  ([key, value]) =>
                    `${key}: ${Array.isArray(value) ? value.join(", ") : value}`,
                )
                .join(" · ");
              return `<div class="plan-item"><b>${esc(c.title || state.media.find((m) => m.id === c.id)?.title || "Titre")}</b> · ${esc(fields)}</div>`;
            })
            .join("") || '<p class="hint">Aucun changement proposé.</p>'
        }<div class="form-actions"><button class="secondary-button" id="browseMatches">Voir les titres</button><button class="primary-button" id="applyPlan" ${changes.length ? "" : "disabled"}>Appliquer ${changes.length} changement(s)</button></div>`;
      $("#browseMatches").onclick = () => {
        state.unavailableOnly = false;
        state.category = "all";
        state.sagas = false;
        state.matchedIds = matched;
        state.activeCollectionId = null;
        state.collectionKindView = null;
        $("#searchInput").value = "";
        $("#assistantDialog").close();
        render();
        toast(`${matched.size} titre(s) correspondent à la demande.`);
      };
      $("#applyPlan").onclick = applyPlan;
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      b.disabled = false;
    }
  }
  async function applyPlan() {
    if (!state.plan) return;
    if (state.busy.has("series-metadata"))
      return toast("Attendez la fin des corrections de la série.", true);
    const b = $("#applyPlan");
    setBusy("apply-plan", true);
    b.disabled = true;
    try {
      const changed = await invoke("apply_plan", { plan: state.plan });
      const byId = new Map(changed.map((m) => [m.id, m]));
      state.media = state.media.map((m) => byId.get(m.id) || m);
      await reloadCollections();
      populateFilters();
      render();
      $("#assistantDialog").close();
      toast("Organisation appliquée.");
    } catch (err) {
      toast(errorText(err), true);
    } finally {
      setBusy("apply-plan", false);
      b.disabled = false;
    }
  }
  function bind() {
    $("#unavailableButton").onclick = () => {
      state.unavailableOnly = true;
      state.category = "all";
      state.sagas = false;
      state.activeCollectionId = null;
      state.collectionKindView = null;
      state.matchedIds = null;
      state.watchedFilter = "all";
      $("#searchInput").value = "";
      $("#genreFilter").value = "";
      $("#yearFilter").value = "";
      $$(".nav-item.active").forEach((b) => b.classList.remove("active"));
      render();
    };
    $("#confirmRemoveUnavailable").onclick = removeUnavailable;
    $$(".nav-item[data-category]").forEach((b) =>
      b.addEventListener("click", () => {
        state.category = b.dataset.category;
        state.activeSeriesName = null;
        state.unavailableOnly = false;
        state.sagas = false;
        state.activeCollectionId = null;
        state.collectionKindView = null;
        state.matchedIds = null;
        $$(".nav-item[data-category]").forEach((x) =>
          x.classList.toggle("active", x === b),
        );
        $("#sagasButton").classList.remove("active");
        render();
      }),
    );
    $("#sagasButton").onclick = () => {
      state.unavailableOnly = false;
      state.sagas = true;
      state.activeCollectionId = null;
      state.collectionKindView = "saga";
      state.category = "all";
      state.matchedIds = null;
      $$(".nav-item[data-category]").forEach((x) =>
        x.classList.remove("active"),
      );
      $("#sagasButton").classList.toggle("active", state.sagas);
      render();
    };
    $("#playlistsButton").onclick = () => {
      state.unavailableOnly = false;
      state.sagas = false;
      state.category = "all";
      state.activeCollectionId = null;
      state.collectionKindView = "playlist";
      state.matchedIds = null;
      render();
    };
    $("#createSagaButton").onclick = () => openCollectionDialog("saga");
    $("#sagaRenameForm").onsubmit = (event) => {
      event.preventDefault();
      const name = $("#sagaName").value.trim();
      if (name) mutateSaga("rename_saga", { name });
    };
    $("#sagaImageImport").onclick = () => mutateSaga("import_saga_poster");
    $("#sagaImageReset").onclick = () => mutateSaga("reset_saga_poster");
    $("#sagaImageSearchForm").onsubmit = (event) => {
      event.preventDefault();
      searchSagaArtwork();
    };
    $("#createPlaylistButton").onclick = () => openCollectionDialog("playlist");
    $("#collectionForm").onsubmit = (event) => {
      event.preventDefault();
      const draft = state.collectionDraft;
      if (draft)
        createCollection(draft.kind, $("#collectionName").value, draft.mediaId);
    };
    $("#confirmCollectionDelete").onclick = deleteCollection;
    for (const selector of [
      "#collectionPickerSearch",
      "#collectionPickerCategory",
      "#collectionPickerGenre",
      "#collectionPickerYear",
      "#collectionPickerWatched",
    ]) {
      $(selector).addEventListener(
        selector === "#collectionPickerSearch" ? "input" : "change",
        () => {
          if (!state.collectionPicker) return;
          state.collectionPicker.page = 0;
          renderCollectionPicker();
        },
      );
    }
    $("#collectionPickerPrevious").onclick = () => {
      if (!state.collectionPicker || state.busy.has("collections")) return;
      state.collectionPicker.page = Math.max(
        0,
        state.collectionPicker.page - 1,
      );
      renderCollectionPicker();
    };
    $("#collectionPickerNext").onclick = () => {
      if (!state.collectionPicker || state.busy.has("collections")) return;
      state.collectionPicker.page += 1;
      renderCollectionPicker();
    };
    $("#collectionPickerClear").onclick = () => {
      if (!state.collectionPicker || state.busy.has("collections")) return;
      state.collectionPicker.selected.clear();
      renderCollectionPicker();
    };
    $("#collectionPickerConfirm").onclick = addPickedVideos;
    $("#searchInput").oninput = render;
    $("#genreFilter").onchange = render;
    $("#yearFilter").onchange = render;
    $("#watchedFilter").onchange = (event) => {
      state.watchedFilter = event.currentTarget.value;
      render();
    };
    $("#sortFilter").onchange = (event) => {
      const collection = state.collections.find(
        (c) => c.id === state.activeCollectionId,
      );
      if (collection?.kind === "saga")
        state.collectionSort = event.currentTarget.value;
      else if (collection?.kind !== "playlist")
        state.sort = event.currentTarget.value;
      render();
    };
    $("#clearAssistantFilter").onclick = () => {
      state.matchedIds = null;
      render();
    };
    $("#thumbnailSize").oninput = (event) => {
      state.settings.thumbnail_size = Number(event.target.value);
      applyTheme();
      persistPresentation();
    };
    $$(".view-toggle button").forEach(
      (b) =>
        (b.onclick = () => {
          state.settings.view = b.dataset.view;
          render();
          persistPresentation();
        }),
    );
    $("#refreshButton").dataset.busy = "refresh";
    $("#refreshButton").onclick = refresh;
    $("#assistantButton").onclick = openAssistant;
    $("#settingsButton").onclick = openSettings;
    $("#settingsForm").onsubmit = saveSettings;
    $("#folderButton").onclick = selectFolder;
    $$("[data-close]").forEach(
      (b) =>
        (b.onclick = () => {
          const dialog = $(`#${b.dataset.close}`);
          if (b.dataset.close === "detailsDialog") {
            state.detailsRequest += 1;
            state.activeDetailsId = null;
          }
          dialog.close();
        }),
    );
    document.addEventListener("keydown", (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        $(
          $("#collectionPickerDialog").open
            ? "#collectionPickerSearch"
            : "#searchInput",
        ).focus();
      }
      if ((e.metaKey || e.ctrlKey) && e.key === ",") {
        e.preventDefault();
        openSettings();
      }
    });
    // Only internal playlist ordering is supported; never navigate to dropped files/URLs.
    document.addEventListener("dragover", (event) => event.preventDefault());
    document.addEventListener("drop", (event) => event.preventDefault());
  }
  bind();
  $("#enrichCatalogueButton").onclick = enrichCatalogue;
  bootstrap();
})();
