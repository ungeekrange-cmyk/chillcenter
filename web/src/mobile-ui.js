"use strict";
/* Smartphone navigation lives only in the web build.  It forwards to the
 * existing desktop controls so shared shell state and the virtual catalogue
 * remain the single source of truth. */
(function mobileUi(root, doc) {
  const mobileQuery = root.matchMedia?.("(max-width: 760px), (max-height: 500px) and (pointer: coarse)");
  const isPhone = () => Boolean(mobileQuery?.matches);
  const byId = id => doc.getElementById(id);
  // These web-only labels are deliberately not added to shared locales.  The
  // normal catalogue remains authoritative; this small fallback covers only
  // labels the desktop client never renders.
  const webEnglish = { "Téléchargés": "Downloads", "Plus": "More", "Navigation mobile": "Mobile navigation", "Plus de rubriques": "More sections", "Rechercher et filtrer": "Search and filter", "Fermer la recherche": "Close search" };
  const t = text => {
    const translated = root.chillI18n?.t?.(text) || text;
    return translated === text && root.chillI18n?.language === "en" ? (webEnglish[text] || text) : translated;
  };
  const icon = (path, strokeWidth = 2) => `<svg aria-hidden="true" viewBox="0 0 24 24" focusable="false"><path d="${path}" fill="none" stroke="currentColor" stroke-width="${strokeWidth}" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  const paths = {
    home: "M3.5 10.5 11.2 4a1.25 1.25 0 0 1 1.6 0l7.7 6.5 M5.8 8.6v9.6a1.3 1.3 0 0 0 1.3 1.3H10v-4.2a2 2 0 0 1 4 0v4.2h2.9a1.3 1.3 0 0 0 1.3-1.3V8.6",
    film: "M4.5 10.2H19a1.25 1.25 0 0 1 1.25 1.25v6.5A1.25 1.25 0 0 1 19 19.2H5.75a1.25 1.25 0 0 1-1.25-1.25V10.2Zm0 0-.9-3.7a1 1 0 0 1 .75-1.2L18.5 2.9a1 1 0 0 1 1.15.8l.6 3.5L4.5 10.2Zm3-5.42L9.6 9.2m3.45-5.36 2.1 4.27 M7.6 15.45h5.8",
    series: "M8.15 4.5h11.1a1.5 1.5 0 0 1 1.5 1.5v9.4 M5.25 7.8h11.1a1.5 1.5 0 0 1 1.5 1.5v8.5a1.5 1.5 0 0 1-1.5 1.5H5.25a1.5 1.5 0 0 1-1.5-1.5V9.3a1.5 1.5 0 0 1 1.5-1.5Zm4 3.1 4.6 2.65-4.6 2.65Z",
    documentary: "M12 6.7C9.5 4.95 6.65 4.3 3.65 5.05v12.9c3-.75 5.85-.1 8.35 1.65 2.5-1.75 5.35-2.4 8.35-1.65V5.05c-3-.75-5.85-.1-8.35 1.65Zm0 0v12.9 M6.3 8.5c1.12.05 2.18.35 3.2.9m8.2-.9c-1.12.05-2.18.35-3.2.9 M6.3 12.2c1.12.05 2.18.35 3.2.9m8.2-.9c-1.12.05-2.18.35-3.2.9",
    show: "M3.25 4.65h17.5 M4.7 4.65v14.7m14.6-14.7v14.7 M10 4.65c-.35 3.7-1.95 6.4-5.3 7.8 1.9 1.3 3 3.6 3.25 6.9 M14 4.65c.35 3.7 1.95 6.4 5.3 7.8-1.9 1.3-3 3.6-3.25 6.9 M3.25 19.35h17.5",
    download: "M12 3.75v11.1m-4.05-4.1L12 14.85l4.05-4.1 M4.4 15.2v3.1a1.55 1.55 0 0 0 1.55 1.55h12.1a1.55 1.55 0 0 0 1.55-1.55v-3.1",
    sagas: "m10.2 16.85-1.45 1.45a3.6 3.6 0 0 1-5.09-5.09l3.63-3.63a3.6 3.6 0 0 1 5.09 0 M13.8 7.15l1.45-1.45a3.6 3.6 0 0 1 5.09 5.09l-3.63 3.63a3.6 3.6 0 0 1-5.09 0 M9.5 14.5l5-5",
    playlists: "M4.2 6.3h15.6 M4.2 11h9.7m-9.7 4.7h7.4 M15.5 12.65l5.05 3.15-5.05 3.15z",
    more: "M5 12h.01M12 12h.01M19 12h.01",
  };
  const navigationStroke = { strokeWidth: 1.45 };
  const item = (attributes, label, path, { shortLabel = label, strokeWidth = 2 } = {}) =>
    `<button type="button" ${attributes} aria-label="${t(label)}">${icon(path, strokeWidth)}<span>${t(shortLabel)}</span></button>`;

  function forward(selector) {
    const source = doc.querySelector(selector);
    if (source && !source.hidden && !source.disabled) {
      closeSheet();
      setDiscoveryOpen(false);
      source.click();
    }
  }
  function closeSheet() { const sheet = byId("mobile-more-sheet"); if (sheet?.open) sheet.close(); }
  function addMobileShell() {
    if (byId("mobile-nav")) return;
    const topbar = doc.querySelector(".app-topbar"), desktopLogo = byId("brand-logo");
    if (topbar && desktopLogo && !byId("mobile-brand")) {
      const brand = doc.createElement("button");
      brand.id = "mobile-brand"; brand.type = "button"; brand.className = "mobile-brand";
      brand.setAttribute("aria-label", t("Accueil"));
      const image = desktopLogo.cloneNode(true); image.removeAttribute("id"); image.alt = "";
      brand.append(image); brand.addEventListener("click", () => forward('#show-home'));
      topbar.prepend(brand);
    }
    if (topbar && !byId("mobile-search-toggle")) {
      const searchToggle = doc.createElement("button");
      searchToggle.id = "mobile-search-toggle"; searchToggle.type = "button"; searchToggle.className = "mobile-search-toggle";
      searchToggle.setAttribute("aria-label", t("Rechercher et filtrer")); searchToggle.setAttribute("aria-expanded", "false"); searchToggle.setAttribute("aria-controls", "search filter-form");
      searchToggle.innerHTML = icon("m10.5 4a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13m5 11 4 4");
      searchToggle.addEventListener("click", () => setDiscoveryOpen(!doc.body.classList.contains("mobile-discovery-open"), searchToggle));
      topbar.append(searchToggle);
    }
    const nav = doc.createElement("nav");
    nav.id = "mobile-nav"; nav.className = "mobile-nav"; nav.setAttribute("aria-label", t("Navigation mobile"));
    nav.innerHTML =
      item("data-mobile-home", "Accueil", paths.home, navigationStroke)
      + item('data-mobile-category="film"', "Films", paths.film, navigationStroke)
      + item('data-mobile-category="serie"', "Séries", paths.series, navigationStroke)
      + item("data-mobile-downloads hidden", "Téléchargements", paths.download, { ...navigationStroke, shortLabel: "Téléchargés" })
      + item('data-mobile-more aria-haspopup="dialog" aria-controls="mobile-more-sheet"', "Plus", paths.more);
    doc.body.append(nav);
    nav.querySelector("[data-mobile-more]").addEventListener("click", () => byId("mobile-more-sheet")?.showModal());

    const sheet = doc.createElement("dialog");
    sheet.id = "mobile-more-sheet"; sheet.className = "mobile-more-sheet"; sheet.setAttribute("aria-label", t("Plus de rubriques"));
    sheet.innerHTML = `<div class="mobile-sheet-handle" aria-hidden="true"></div><div class="mobile-sheet-heading"><strong>${t("Bibliothèque")}</strong><button type="button" class="secondary" data-mobile-close aria-label="${t("Fermer")}">×</button></div><div class="mobile-sheet-actions">`
      + item('data-mobile-category="documentaire"', "Documentaires", paths.documentary, navigationStroke)
      + item('data-mobile-category="spectacle"', "Spectacles", paths.show, navigationStroke)
      + item('data-mobile-category=""', "Toute la collection", "M4 4h16v16H4z M12 4v16 M4 12h16")
      + item("data-mobile-sagas", "Sagas", paths.sagas, navigationStroke)
      + item("data-mobile-playlists", "Listes de lecture", paths.playlists, navigationStroke)
      + item("data-mobile-config", "Configuration", "M12 3v2m0 14v2M3 12h2m14 0h2M5.6 5.6l1.4 1.4m10 10 1.4 1.4M18.4 5.6L17 7M7 17l-1.4 1.4 M9 12a3 3 0 1 0 6 0 3 3 0 1 0-6 0")
      + "</div>";
    doc.body.append(sheet);
    for (const button of doc.querySelectorAll("[data-mobile-category]")) {
      button.addEventListener("click", () => forward(`[data-category="${button.dataset.mobileCategory}"]`));
    }
    for (const [name, target] of [["home","#show-home"],["downloads","#show-downloads"],["sagas","#show-sagas"],["playlists","#show-playlists"],["config","#admin-toggle"]]) {
      doc.querySelector(`[data-mobile-${name}]`).addEventListener("click", () => forward(target));
    }
    sheet.querySelector("[data-mobile-close]").addEventListener("click", closeSheet);
    sheet.addEventListener("click", event => { if (event.target === sheet) closeSheet(); });
  }
  function setDiscoveryOpen(open, focus) {
    // Opening is meaningful only on a phone; closing must still clear stale
    // state after a resize to desktop, ready for the next phone viewport.
    if (open && !isPhone()) return;
    doc.body.classList.toggle("mobile-discovery-open", open);
    const toggle = byId("mobile-search-toggle"); toggle?.setAttribute("aria-expanded", String(open));
    if (open) root.setTimeout(() => byId("search")?.focus(), 0); else focus?.focus?.();
  }
  function syncShell() {
    const nav = byId("mobile-nav"); if (!nav) return;
    const shown = id => byId(id)?.hidden === false;
    const category = shown("catalog-panel") ? byId("category")?.value : null;
    const active = doc.querySelector('[data-category="film"]')?.disabled === false;
    const hasDownloads = shown("show-downloads");
    nav.hidden = !isPhone() || (!active && !hasDownloads) || doc.body.classList.contains("player-open");
    const searchToggle = byId("mobile-search-toggle");
    if (searchToggle) searchToggle.hidden = !active;

    const detailsAreSeries = shown("collection-panel") && byId("collection-members")?.classList.contains("series-members");
    const selected = shown("home-panel") ? "home"
      : shown("downloads-panel") ? "downloads"
      : detailsAreSeries || category === "serie" ? "series"
      : category === "film" ? "films" : "more";
    for (const [key, selector] of [
      ["home","[data-mobile-home]"], ["films",'[data-mobile-category="film"]'],
      ["series",'[data-mobile-category="serie"]'], ["downloads","[data-mobile-downloads]"],
      ["more","[data-mobile-more]"],
    ]) {
      const button = nav.querySelector(selector);
      const available = key === "downloads" ? hasDownloads : active;
      button.hidden = !available;
      button.classList.toggle("active", available && key === selected);
      button.setAttribute("aria-current", available && key === selected ? "page" : "false");
    }
  }
  function installVirtualizer() {
    if (typeof root.catalogWindow !== "function" || root.__chillDesktopCatalogWindow) return;
    const desktop = root.catalogWindow;
    root.__chillDesktopCatalogWindow = desktop;
    root.catalogWindow = (count, width, posterSize, scrollTop, viewportHeight, list, infoHeight) => {
      const useMobile = isPhone() && !list;
      // Three posters are the phone default. At a 320px viewport two columns
      // preserve independent 44px seen/add targets. Compare viewport width,
      // not the smaller content width (390px phones have ~358px of content).
      // A mathematically exact three-column width can round to a ratio of
      // 2.9999999999999996 before the shared floor(), especially with cutout
      // insets/subpixel resizing. Bias only that rounding error, not the layout.
      const epsilon = Math.max(1, width) * Number.EPSILON * 8;
      const preferredColumns = root.innerWidth <= 360 ? 2 : 3;
      const columns = infoHeight === undefined ? preferredColumns : Math.min(preferredColumns, Math.max(1, Math.floor((width + 18) / 114)));
      const adapted = useMobile ? Math.max(1, (width - (columns - 1) * 18) / columns - epsilon) : posterSize;
      return desktop(count, width, adapted, scrollTop, viewportHeight, list, infoHeight);
    };
  }
  function install() {
    addMobileShell(); installVirtualizer();
    const original = root.renderShell;
    if (typeof original === "function" && !original.__mobileWrapped) {
      const wrapped = (...args) => { const result = original(...args); syncShell(); return result; };
      wrapped.__mobileWrapped = true; root.renderShell = wrapped;
    }
    root.addEventListener("resize", () => { root.restoreCatalogLayout?.(); syncShell(); }, { passive: true });
    mobileQuery?.addEventListener?.("change", () => { closeSheet(); setDiscoveryOpen(false); root.restoreCatalogLayout?.(); syncShell(); });
    // player.js toggles this class without re-rendering the shell. Observe it
    // so a resize during playback cannot leave the bottom navigation hidden
    // when the player subsequently closes.
    new MutationObserver(() => {
      if (doc.body.classList.contains("player-open")) closeSheet();
      syncShell();
    }).observe(doc.body, { attributes: true, attributeFilter: ["class"] });
    const observeDownloads = () => {
      const source = byId("show-downloads");
      if (source) new MutationObserver(syncShell).observe(source, { attributes: true, attributeFilter: ["hidden"] });
      syncShell();
    };
    // The shared downloads script is deferred at the end of the page.
    if (byId("show-downloads")) observeDownloads();
    else doc.addEventListener("DOMContentLoaded", observeDownloads, { once: true });
    doc.addEventListener("keydown", event => { if (event.key === "Escape" && doc.body.classList.contains("mobile-discovery-open")) { event.preventDefault(); setDiscoveryOpen(false, byId("mobile-search-toggle")); } });
    root.chillReady?.then(() => { root.restoreCatalogLayout?.(); syncShell(); });
    syncShell();
  }
  root.ChillMobileUi = { isPhone, install, syncShell };
  if (doc.readyState === "loading") doc.addEventListener("DOMContentLoaded", install, { once: true }); else install();
})(window, document);
