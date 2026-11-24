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
  const icon = path => `<svg aria-hidden="true" viewBox="0 0 24 24" focusable="false"><path d="${path}" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  const paths = {
    home: "m3 10 9-7 9 7 M5 9v11h5v-6h4v6h5V9",
    film: "M4 10h16v10H4z M4 10 3 5l16-3 1 5-16 3 M7 4l3 4m3-5 3 4",
    series: "M7 4h13v13 M4 8h13v12H4z M9 12l4 2-4 2z",
    documentary: "M12 6c-3-2-6-3-9-2v14c3-1 6 0 9 2 3-2 6-3 9-2V4c-3-1-6 0-9 2zm0 0v14 M6 8l3 1m6 0 3-1 M6 12l3 1m6 0 3-1",
    show: "M3 4h18 M4 4v15m16-15v15 M10 4c0 5-2 8-6 9 2 0 4 2 4 6m6-15c0 5 2 8 6 9-2 0-4 2-4 6 M3 20h18",
    download: "M12 3v12m-4-4 4 4 4-4 M4 16v4h16v-4",
    more: "M5 12h.01M12 12h.01M19 12h.01",
  };
  const item = (attributes, label, path, shortLabel = label) =>
    `<button type="button" ${attributes} aria-label="${t(label)}">${icon(path)}<span>${t(shortLabel)}</span></button>`;

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
      item("data-mobile-home", "Accueil", paths.home)
      + item('data-mobile-category="film"', "Films", paths.film)
      + item('data-mobile-category="serie"', "Séries", paths.series)
      + item("data-mobile-downloads hidden", "Téléchargements", paths.download, "Téléchargés")
      + item('data-mobile-more aria-haspopup="dialog" aria-controls="mobile-more-sheet"', "Plus", paths.more);
    doc.body.append(nav);
    nav.querySelector("[data-mobile-more]").addEventListener("click", () => byId("mobile-more-sheet")?.showModal());

    const sheet = doc.createElement("dialog");
    sheet.id = "mobile-more-sheet"; sheet.className = "mobile-more-sheet"; sheet.setAttribute("aria-label", t("Plus de rubriques"));
    sheet.innerHTML = `<div class="mobile-sheet-handle" aria-hidden="true"></div><div class="mobile-sheet-heading"><strong>${t("Bibliothèque")}</strong><button type="button" class="secondary" data-mobile-close aria-label="${t("Fermer")}">×</button></div><div class="mobile-sheet-actions">`
      + item('data-mobile-category="documentaire"', "Documentaires", paths.documentary)
      + item('data-mobile-category="spectacle"', "Spectacles", paths.show)
      + item('data-mobile-category=""', "Toute la collection", "M4 4h16v16H4z M12 4v16 M4 12h16")
      + item("data-mobile-sagas", "Sagas", "M4 5h16v14H4z M8 5v14")
      + item("data-mobile-playlists", "Listes de lecture", "M5 5h14M5 12h14M5 19h14")
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
