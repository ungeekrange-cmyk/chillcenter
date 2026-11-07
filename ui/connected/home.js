"use strict";
// A profile-scoped, bounded shelf read; catalogue browsing stays separate.
(() => {
  let generation = 0, pending = null, snapshot = null;
  const context = () => state?.stage === "connected" && state.server_id && state.profile_id ? state.server_id + ":" + state.profile_id : "";
  const visible = () => shellSection === "home" && !document.hidden && !document.body.classList.contains("player-open");
  function clearRails() {
    window.catalogRetireRecommendationRail?.($("home-rails"));
    $("home-rails").replaceChildren();
  }
  function rows(value) {
    const seen = new Set();
    return value.filter(item => {
      if (!item?.media || typeof item.media.id !== "string") return false;
      const series = item.media.category === "serie";
      if (series && (!item.series_key || item.media.season != null || item.media.episode != null)) return false;
      const key = series ? "series:" + item.series_key : "media:" + item.media.id;
      if (seen.has(key)) return false;
      seen.add(key); return true;
    }).slice(0, 20);
  }
  function badge(item) {
    const values = [];
    if (Number.isInteger(item.season) && item.season >= 0) values.push(uiText("Saison ") + item.season);
    else if (item.series_key) values.push(uiText("Saison inconnue"));
    const p = item.progress;
    if (p && Number.isSafeInteger(p.position_ms) && Number.isSafeInteger(p.duration_ms) && p.position_ms >= 0 && p.duration_ms > 0) values.push(Math.min(100, Math.floor(p.position_ms / p.duration_ms * 100)) + " %");
    return values.join(" · ");
  }
  function render(data, key) {
    clearRails();
    for (const [title, field] of [["Reprendre", "continue_watching"], ["Ajouts récents", "recently_added"], ["Pour vous", "recommendations"]]) {
      const list = rows(data[field]); if (!list.length) continue;
      const label = uiText(title), byId = new Map(list.map(item => [item.media.id, item]));
      const rail = window.catalogRecommendationRail?.(list.map(item => ({ ...item.media, series_key: item.series_key, matches: item.matches || [] })), {
        label, previousLabel: uiText("Éléments précédents — {0}", label), nextLabel: uiText("Éléments suivants — {0}", label),
        beforeOpen(item) {
          if (context() !== key || !visible()) return false;
          if (item.series_key) {
            if (field === "continue_watching") void window.catalogOpenSeries?.(item.series_key, byId.get(item.id)?.season);
            else void window.catalogOpenSeries?.(item.series_key);
            return false;
          }
        }
      });
      if (!rail) continue;
      if (field === "continue_watching") for (const card of rail.querySelectorAll(".recommendation-card")) {
        const item = byId.get(card.dataset.homeMediaId), text = item && badge(item);
        if (text) card.append(node("span", text, "home-badge"));
      }
      $("home-rails").append(rail);
    }
    $("home-empty").hidden = Boolean($("home-rails").children.length);
  }
  async function refresh(force = false) {
    const key = context(); if (!key || !visible()) return true;
    if (pending?.key === key) { if (force) pending.again = true; return pending.promise; }
    const task = { key, generation, again: false, promise: null }; pending = task;
    const current = () => pending === task && generation === task.generation && context() === key;
    task.promise = (async () => {
      try {
        const data = await invoke({ action: "home" });
        if (!current() || !visible() || data?.discarded) return false;
        if (!["continue_watching", "recently_added", "recommendations"].every(field => Array.isArray(data?.[field]))) throw Error(uiText("Accueil indisponible."));
        snapshot = { key, data }; render(data, key); $("home-status").textContent = ""; return true;
      } catch (error) {
        if (!current()) return false;
        if (error?.status === 404 || error?.status === 405) {
          snapshot = null; clearRails(); $("home-empty").hidden = true;
          $("home-status").textContent = uiText("L’accueil nécessite une mise à jour du serveur. La recherche et les catégories restent disponibles.");
        } else {
          if (snapshot?.key === key) render(snapshot.data, key);
          $("home-status").textContent = uiText(snapshot?.key === key ? "Serveur indisponible. Le dernier accueil est conservé." : "Accueil indisponible. Réessayez ou utilisez les catégories.");
        }
        return false;
      } finally {
        if (pending === task) { pending = null; if (task.again && generation === task.generation && context() === key && visible()) void refresh(); }
      }
    })();
    return task.promise;
  }
  window.refreshHome = refresh;
  window.clearHomeContext = () => { generation++; pending = snapshot = null; clearRails(); $("home-status").textContent = ""; $("home-empty").hidden = true; };
  window.homeVisibilityChanged = shown => window.catalogSetRecommendationRailVisible?.($("home-rails"), shown);
  window.refreshHomeAfterMutation = () => { if (visible()) void refresh(true); };
  document.addEventListener("visibilitychange", () => { if (visible()) void refresh(); });
})();
