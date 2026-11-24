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
  const item = (attributes, label, glyph, shortLabel = label) =>
    `<button type="button" ${attributes} aria-label="${t(label)}"><svg class="nav-icon" aria-hidden="true" viewBox="0 0 24 24" focusable="false"><use href="#nav-glyph-${glyph}"/></svg><span>${t(shortLabel)}</span></button>`;

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
      searchToggle.innerHTML = '<svg aria-hidden="true" viewBox="0 0 24 24" focusable="false"><path d="m10.5 4a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13m5 11 4 4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>';
      searchToggle.addEventListener("click", () => setDiscoveryOpen(!doc.body.classList.contains("mobile-discovery-open"), searchToggle));
      topbar.append(searchToggle);
    }
    const nav = doc.createElement("nav");
    nav.id = "mobile-nav"; nav.className = "mobile-nav"; nav.setAttribute("aria-label", t("Navigation mobile"));
    nav.innerHTML =
      item("data-mobile-home", "Accueil", "home")
      + item('data-mobile-category="film"', "Films", "film")
      + item('data-mobile-category="serie"', "Séries", "series")
      + item("data-mobile-downloads hidden", "Téléchargements", "download", "Téléchargés")
      + item('data-mobile-more aria-haspopup="dialog" aria-controls="mobile-more-sheet"', "Plus", "more");
    doc.body.append(nav);
    nav.querySelector("[data-mobile-more]").addEventListener("click", () => byId("mobile-more-sheet")?.showModal());

    const sheet = doc.createElement("dialog");
    sheet.id = "mobile-more-sheet"; sheet.className = "mobile-more-sheet"; sheet.setAttribute("aria-label", t("Plus de rubriques"));
    sheet.innerHTML = `<div class="mobile-sheet-handle" aria-hidden="true"></div><div class="mobile-sheet-heading"><strong>${t("Bibliothèque")}</strong><button type="button" class="secondary" data-mobile-close aria-label="${t("Fermer")}">×</button></div><div class="mobile-sheet-actions">`
      + item('data-mobile-category="documentaire"', "Documentaires", "documentary")
      + item('data-mobile-category="spectacle"', "Spectacles", "show")
      + item('data-mobile-category=""', "Toute la collection", "library")
      + item("data-mobile-sagas", "Sagas", "sagas")
      + item("data-mobile-playlists", "Listes de lecture", "playlists")
      + item("data-mobile-config", "Configuration", "settings")
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
    new MutationObserver(syncShell).observe(byId("show-downloads"), { attributes: true, attributeFilter: ["hidden"] });
    doc.addEventListener("keydown", event => { if (event.key === "Escape" && doc.body.classList.contains("mobile-discovery-open")) { event.preventDefault(); setDiscoveryOpen(false, byId("mobile-search-toggle")); } });
    root.chillReady?.then(() => { root.restoreCatalogLayout?.(); syncShell(); });
    syncShell();
  }
  root.ChillMobileUi = { isPhone, install, syncShell };
  if (doc.readyState === "loading") doc.addEventListener("DOMContentLoaded", install, { once: true }); else install();
})(window, document);
