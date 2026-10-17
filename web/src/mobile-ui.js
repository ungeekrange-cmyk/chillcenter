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
  const webEnglish = { "Docs": "Documentaries", "Plus": "More", "Navigation mobile": "Mobile navigation", "Plus de rubriques": "More sections", "Rechercher et filtrer": "Search and filter", "Fermer la recherche": "Close search", "Plus d’informations sur la saison": "More information about this season", "Plus d’informations sur la série": "More information about this series" };
  const t = text => {
    const translated = root.chillI18n?.t?.(text) || text;
    return translated === text && root.chillI18n?.language === "en" ? (webEnglish[text] || text) : translated;
  };
  const icon = path => `<svg aria-hidden="true" viewBox="0 0 24 24" focusable="false"><path d="${path}" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  const entries = [
    ["film", "Films", "M4 5h16M4 12h16M4 19h16"],
    ["serie", "Séries", "M5 4h14v16H5zM9 4v16M15 4v16"],
    ["documentaire", "Docs", "M4 5h16v14H4zM8 5v14M16 5v14"],
    ["spectacle", "Spectacles", "M4 5h16v14H4zM8 5l8 7M16 5l-8 7"],
  ];

  function forward(selector) {
    const source = doc.querySelector(selector);
    if (source && !source.disabled) { setDiscoveryOpen(false); source.click(); }
  }
  function closeSheet() { const sheet = byId("mobile-more-sheet"); if (sheet?.open) sheet.close(); }
  function addMobileShell() {
    if (byId("mobile-nav")) return;
    const topbar = doc.querySelector(".app-topbar"), desktopLogo = byId("brand-logo");
    if (topbar && desktopLogo && !byId("mobile-brand")) {
      const brand = doc.createElement("button");
      brand.id = "mobile-brand"; brand.type = "button"; brand.className = "mobile-brand";
      brand.setAttribute("aria-label", t("Toute la collection"));
      const image = desktopLogo.cloneNode(true); image.removeAttribute("id"); image.alt = "";
      brand.append(image); brand.addEventListener("click", () => forward('[data-category=""]'));
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
    nav.innerHTML = entries.map(([category, label, path]) => `<button type="button" data-mobile-category="${category}" aria-label="${t(label)}">${icon(path)}<span>${t(label)}</span></button>`).join("") + `<button type="button" data-mobile-more aria-haspopup="dialog" aria-label="${t("Plus")}">${icon("M5 12h.01M12 12h.01M19 12h.01")}<span>${t("Plus")}</span></button>`;
    doc.body.append(nav);
    for (const button of nav.querySelectorAll("[data-mobile-category]")) button.addEventListener("click", () => forward(`[data-category="${button.dataset.mobileCategory}"]`));
    nav.querySelector("[data-mobile-more]").addEventListener("click", () => byId("mobile-more-sheet")?.showModal());

    const sheet = doc.createElement("dialog");
    sheet.id = "mobile-more-sheet"; sheet.className = "mobile-more-sheet"; sheet.setAttribute("aria-label", t("Plus de rubriques"));
    sheet.innerHTML = `<div class="mobile-sheet-handle" aria-hidden="true"></div><div class="mobile-sheet-heading"><strong>${t("Bibliothèque")}</strong><button type="button" class="secondary" data-mobile-close aria-label="${t("Fermer")}">×</button></div><div class="mobile-sheet-actions"><button type="button" data-mobile-all>${icon("M4 5h16M4 12h16M4 19h16")}<span>${t("Toute la collection")}</span></button><button type="button" data-mobile-sagas>${icon("M4 5h16v14H4zM8 5v14")}<span>${t("Sagas")}</span></button><button type="button" data-mobile-playlists>${icon("M5 5h14M5 12h14M5 19h14")}<span>${t("Listes de lecture")}</span></button><button type="button" data-mobile-config>${icon("M12 3v2m0 14v2M3 12h2m14 0h2M5.6 5.6l1.4 1.4m10 10 1.4 1.4M18.4 5.6L17 7M7 17l-1.4 1.4")}<span>${t("Configuration")}</span></button></div>`;
    doc.body.append(sheet);
    sheet.querySelector("[data-mobile-close]").addEventListener("click", closeSheet);
    sheet.addEventListener("click", event => { if (event.target === sheet) closeSheet(); });
    sheet.querySelector("[data-mobile-all]").addEventListener("click", () => { closeSheet(); forward('[data-category=""]'); });
    sheet.querySelector("[data-mobile-sagas]").addEventListener("click", () => { closeSheet(); forward("#show-sagas"); });
    sheet.querySelector("[data-mobile-playlists]").addEventListener("click", () => { closeSheet(); forward("#show-playlists"); });
    sheet.querySelector("[data-mobile-config]").addEventListener("click", () => { closeSheet(); forward("#admin-toggle"); });
  }
  function setDiscoveryOpen(open, focus) {
    // Opening is meaningful only on a phone; closing must still clear stale
    // state after a resize to desktop, ready for the next phone viewport.
    if (open && !isPhone()) return;
    doc.body.classList.toggle("mobile-discovery-open", open);
    const toggle = byId("mobile-search-toggle"); toggle?.setAttribute("aria-expanded", String(open));
    if (open) root.setTimeout(() => byId("search")?.focus(), 0); else focus?.focus?.();
  }
  function arrangeSeriesDetails() {
    if (!isPhone()) return restoreSeriesDetails();
    const members = byId("collection-members"); if (!members) return;
    for (const season of members.querySelectorAll(".series-season")) {
      const existing = season.querySelector(":scope > .mobile-season-details");
      if (existing) {
        for (const node of Array.from(season.children)) if (node !== existing && node.tagName !== "SUMMARY" && !node.classList.contains("series-episodes")) existing.append(node);
        continue;
      }
      const grid = season.querySelector(":scope > .series-episodes"); if (!grid) continue;
      const extra = doc.createElement("details"); extra.className = "mobile-season-details";
      const summary = doc.createElement("summary"); summary.textContent = t("Plus d’informations sur la saison"); extra.append(summary);
      for (const node of Array.from(season.children)) if (node !== grid && node.tagName !== "SUMMARY") extra.append(node);
      season.append(extra);
    }
    const tools = byId("series-tools");
    if (tools && !tools.querySelector(":scope > .mobile-series-details")) {
      const extra = doc.createElement("details"); extra.className = "mobile-series-details";
      const summary = doc.createElement("summary"); summary.textContent = t("Plus d’informations sur la série"); extra.append(summary);
      for (const node of Array.from(tools.childNodes)) if (node !== extra) extra.append(node);
      tools.append(extra);
    }
  }
  function restoreSeriesDetails() {
    const members = byId("collection-members");
    for (const extra of members?.querySelectorAll?.(".mobile-season-details") || []) {
      const season = extra.parentElement, grid = season?.querySelector(":scope > .series-episodes");
      for (const node of Array.from(extra.children)) if (node.tagName !== "SUMMARY") {
        if (node.classList?.contains("series-poster")) season.append(node); else season.insertBefore(node, grid);
      }
      extra.remove();
    }
    const tools = byId("series-tools"), extra = tools?.querySelector(":scope > .mobile-series-details");
    if (extra) { for (const node of Array.from(extra.childNodes)) if (node.tagName !== "SUMMARY") tools.insertBefore(node, extra); extra.remove(); }
  }
  function syncShell() {
    const nav = byId("mobile-nav"); if (!nav) return;
    const category = byId("category")?.value || "";
    // `state` is a top-level lexical binding in the shared client, not a
    // window property. Its navigation buttons already express whether a
    // profile is active, without reaching across that boundary.
    const sharedFilm = doc.querySelector('[data-category="film"]');
    const active = Boolean(sharedFilm && !sharedFilm.disabled);
    nav.hidden = !isPhone() || !active || doc.body.classList.contains("player-open");
    const searchToggle = byId("mobile-search-toggle");
    if (searchToggle) searchToggle.hidden = !active;
    for (const button of nav.querySelectorAll("[data-mobile-category]")) {
      const selected = button.dataset.mobileCategory === category && !byId("catalog-panel")?.hidden;
      button.classList.toggle("active", selected); button.setAttribute("aria-current", selected ? "page" : "false"); button.disabled = !active;
    }
    const more = nav.querySelector("[data-mobile-more]");
    const nonCatalog = !byId("collections-panel")?.hidden || !byId("collection-panel")?.hidden || !byId("picker-panel")?.hidden || !byId("admin-panel")?.hidden;
    const selectedMore = Boolean(nonCatalog) || (category === "" && !byId("catalog-panel")?.hidden);
    more?.classList.toggle("active", selectedMore); more?.setAttribute("aria-current", selectedMore ? "page" : "false"); more && (more.disabled = !active);
  }
  function installVirtualizer() {
    if (typeof root.catalogWindow !== "function" || root.__chillDesktopCatalogWindow) return;
    const desktop = root.catalogWindow;
    root.__chillDesktopCatalogWindow = desktop;
    root.catalogWindow = (count, width, posterSize, scrollTop, viewportHeight, list) => {
      const useMobile = isPhone() && !list;
      // Three posters are the phone default. At a 320px viewport two columns
      // preserve independent 44px seen/add targets. Compare viewport width,
      // not the smaller content width (390px phones have ~358px of content).
      const adapted = useMobile ? Math.max(1, root.innerWidth <= 360 ? (width - 18) / 2 : (width - 36) / 3) : posterSize;
      return desktop(count, width, adapted, scrollTop, viewportHeight, list);
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
    mobileQuery?.addEventListener?.("change", () => { closeSheet(); setDiscoveryOpen(false); root.restoreCatalogLayout?.(); arrangeSeriesDetails(); syncShell(); });
    // player.js toggles this class without re-rendering the shell. Observe it
    // so a resize during playback cannot leave the bottom navigation hidden
    // when the player subsequently closes.
    new MutationObserver(() => {
      if (doc.body.classList.contains("player-open")) closeSheet();
      syncShell();
    }).observe(doc.body, { attributes: true, attributeFilter: ["class"] });
    let seriesQueued = false;
    new MutationObserver(() => { if (!seriesQueued) { seriesQueued = true; root.queueMicrotask(() => { seriesQueued = false; arrangeSeriesDetails(); }); } }).observe(byId("collection-panel"), { childList: true, subtree: true });
    doc.addEventListener("keydown", event => { if (event.key === "Escape" && doc.body.classList.contains("mobile-discovery-open")) { event.preventDefault(); setDiscoveryOpen(false, byId("mobile-search-toggle")); } });
    root.chillReady?.then(() => { root.restoreCatalogLayout?.(); syncShell(); });
    syncShell();
  }
  root.ChillMobileUi = { isPhone, install, syncShell, arrangeSeriesDetails };
  if (doc.readyState === "loading") doc.addEventListener("DOMContentLoaded", install, { once: true }); else install();
})(window, document);
