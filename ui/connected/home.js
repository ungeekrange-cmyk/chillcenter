"use strict";
// A profile-scoped, bounded shelf read; catalogue browsing stays separate.
(() => {
  let generation = 0, pending = null, snapshot = null;
  const mutations = new Set();
  const rendered = new Map();
  const context = () => state?.stage === "connected" && state.server_id && state.profile_id ? state.server_id + ":" + state.profile_id : "";
  const visible = () => shellSection === "home" && !document.hidden && !document.body.classList.contains("player-open");
  function clearRails() {
    rendered.clear();
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
    const count = counts(item), value = percentage(item);
    if (count) values.push(uiText("{0} / {1} épisodes vus", count.watched, count.total));
    if (value !== null) values.push(displayPercent(item,value) + " %");
    else if (Number.isSafeInteger(item.resume_position_ms) && item.resume_position_ms > 0) {
      const seconds = Math.floor(item.resume_position_ms / 1000);
      values.push(uiText("Reprise à {0} · durée indisponible", Math.floor(seconds / 60) + ":" + String(seconds % 60).padStart(2, "0")));
    }
    return values.join(" · ");
  }
  function counts(item) {
    const c = item.episode_counts;
    return item.series_key && c && Number.isSafeInteger(c.watched) && Number.isSafeInteger(c.total) && c.total > 0 && c.watched >= 0 && c.watched <= c.total ? c : null;
  }
  function percentage(item) {
    const c = counts(item); if (c) return c.watched / c.total * 100;
    const p = item.progress;
    return p && Number.isSafeInteger(p.position_ms) && Number.isSafeInteger(p.duration_ms) && p.position_ms >= 0 && p.duration_ms > 0 ? Math.min(100, p.position_ms / p.duration_ms * 100) : null;
  }
  function displayPercent(item,value) {
    const c=counts(item);
    return c ? Math.min(c.watched<c.total?99:100,Math.round(value)) : Math.floor(value);
  }
  const activeMutation = () => [...mutations].some(m => m.key === context() && m.generation === generation);
  const railSignature = (key, field, list, revision) => JSON.stringify([key, field === "continue_watching" ? revision : null, list]);
  async function removeAnimated(wrapper, current, restoreFocus) {
    const rail = wrapper.parentElement, original = [...rail.children], removedIndex = original.indexOf(wrapper), next = original.filter(c => c !== wrapper);
    const focused = wrapper.contains(document.activeElement) || (restoreFocus && document.activeElement === document.body);
    const reduce = globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    if (!reduce && wrapper.animate) await wrapper.animate([{opacity:1},{opacity:0}], {duration:180,easing:"ease-out",fill:"forwards"}).finished.catch(()=>{});
    if (!current()) return;
    const before = next.map(c => c.getBoundingClientRect().left);
    window.catalogRetireRecommendationRail?.(wrapper); wrapper.remove();
    const focusTarget = focused ? next[Math.min(removedIndex,next.length-1)]?.querySelector(".home-watched") : null;
    focusTarget?.focus({preventScroll:true});
    if (!reduce) await Promise.all(next.map((card,i) => {
      const dx = before[i] - card.getBoundingClientRect().left;
      return Number.isFinite(dx) && dx && card.animate ? card.animate([{transform:`translateX(${dx}px)`},{transform:"translateX(0)"}],{duration:300,easing:"cubic-bezier(.2,.8,.2,1)"}).finished.catch(()=>{}) : undefined;
    }));
    if (document.activeElement === focusTarget) focusTarget?.scrollIntoView({block:"nearest",inline:"nearest",behavior:"instant"});
  }
  function resumeCard(card, item, key, revision) {
    const wrapper = node("article", undefined, "home-resume-card"), check = node("button", "○", "home-watched");
    wrapper.dataset.homeMediaId=item.media.id; wrapper.homeSignature=JSON.stringify([item,revision]);
    const poster = card.querySelectorAll(".recommendation-poster")[0], value = percentage(item), text = badge(item);
    if (text) card.append(node("span", text, "home-badge"));
    poster.querySelectorAll(".media-progress")[0]?.remove(); delete poster.dataset.progressMediaId;
    if (value !== null) {
      const bar = node("div", undefined, "home-progress"), fill = node("span");
      bar.setAttribute("role", "progressbar"); bar.setAttribute("aria-label", uiText("Progression de lecture"));
      bar.setAttribute("aria-valuemin", "0"); bar.setAttribute("aria-valuemax", "100"); bar.setAttribute("aria-valuenow", String(displayPercent(item,value)));
      fill.style.width = value + "%"; bar.append(fill); poster.append(bar);
    }
    check.type = "button"; check.setAttribute("aria-pressed", "false");
    check.setAttribute("aria-label", uiText(item.series_key ? "Marquer la saison comme vue : {0}" : "Marquer vu : {0}", item.media.title));
    if (item.series_key && (!Number.isSafeInteger(revision) || revision < 0)) check.disabled = true;
    check.onclick = async () => {
      if (check.disabled || !wrapper.isConnected || context() !== key || !visible()) return;
      const action = {key, generation, restoreFocus:wrapper.contains(document.activeElement)}; mutations.add(action); pending = null; check.disabled = true;
      const current = () => action.generation === generation && context() === key && wrapper.isConnected;
      let saved = false;
      try {
        const command = item.series_key ? {action:"group_watched",media_id:item.media.id,watched:true,scope:"season",expected_revision:revision} : {action:"set_watched",media_id:item.media.id,watched:true};
        const result = await invoke(command);
        if (!current() || result?.discarded) return;
        if (result?.saved !== true) throw Error(uiText("État vu non enregistré. Réessayez."));
        saved = true; check.setAttribute("aria-pressed", "true"); check.textContent = "✓";
        if (snapshot?.key === key) snapshot.data.continue_watching = snapshot.data.continue_watching.filter(i => i.media.id !== item.media.id);
        await removeAnimated(wrapper, current, action.restoreFocus);
        const rail = rendered.get("continue_watching");
        if (rail && snapshot?.key === key) rail.signature = railSignature(key,"continue_watching",rows(snapshot.data.continue_watching),snapshot.data.watch_revision);
        if (context() === key && action.generation === generation) await syncAfterMutation();
      } catch (error) {
        if (current()) { check.disabled = false; check.setAttribute("aria-pressed", "false"); $("home-status").textContent = error.message || uiText("État vu non enregistré. Réessayez."); }
      } finally {
        mutations.delete(action);
        if (saved && context() === key && action.generation === generation && !activeMutation()) void refresh(true);
      }
    };
    wrapper.append(card, check); return wrapper;
  }
  function render(data, key) {
    const focusedId = document.activeElement.closest?.(".home-resume-card")?.dataset.homeMediaId;
    const scrollPositions = new Map([...rendered].map(([field,value]) => [field,value.rail.querySelector(".recommendation-rail")?.scrollLeft || 0]));
    const nextRails = [], nextRendered = new Map();
    for (const [title, field] of [["Reprendre", "continue_watching"], ["Ajouts récents", "recently_added"], ["Pour vous", "recommendations"]]) {
      const list = rows(data[field]); if (!list.length) continue;
      const signature = railSignature(key,field,list,data.watch_revision), existing = rendered.get(field);
      if (existing?.signature === signature) { nextRails.push(existing.rail); nextRendered.set(field,existing); continue; }
      const label = uiText(title), byId = new Map(list.map(item => [item.media.id, item]));
      const rail = window.catalogRecommendationRail?.(list.map(item => ({ ...item.media, series_key: item.series_key, matches: item.matches || [] })), {
        label, previousLabel: uiText("Éléments précédents — {0}", label), nextLabel: uiText("Éléments suivants — {0}", label),
        decorateCard: field === "continue_watching" ? (card, media) => resumeCard(card, byId.get(media.id), key, data.watch_revision) : undefined,
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
      if (field === "continue_watching" && existing) {
        const oldCards=new Map([...existing.rail.querySelectorAll(".home-resume-card")].map(card=>[card.dataset.homeMediaId,card]));
        const viewport=rail.querySelectorAll(".recommendation-rail")[0];
        viewport.replaceChildren(...[...viewport.children].map(card=>{
          const old=oldCards.get(card.dataset.homeMediaId);
          if (!old || old.homeSignature!==card.homeSignature) return card;
          window.catalogRetireRecommendationRail?.(card); return old;
        }));
      }
      nextRails.push(rail); nextRendered.set(field,{rail,signature});
    }
    for (const [field, old] of rendered) if (nextRendered.get(field)?.rail !== old.rail) window.catalogRetireRecommendationRail?.(old.rail);
    rendered.clear(); for (const [field, value] of nextRendered) rendered.set(field,value);
    const target=$("home-rails");
    if (target.insertBefore) {
      for (let i=0;i<nextRails.length;i++) if (target.children[i]!==nextRails[i]) target.insertBefore(nextRails[i],target.children[i]||null);
      for (const child of [...target.children]) if (!nextRails.includes(child)) child.remove();
    } else target.replaceChildren(...nextRails);
    for (const [field,value] of rendered) {
      const viewport=value.rail.querySelector(".recommendation-rail");
      if (viewport) viewport.scrollLeft=scrollPositions.get(field)||0;
    }
    if (focusedId) {
      const card=[...target.querySelectorAll(".home-resume-card")].find(card=>card.dataset.homeMediaId===focusedId);
      card?.querySelector(".home-watched")?.focus({preventScroll:true});
    }
    $("home-empty").hidden = Boolean($("home-rails").children.length);
  }
  async function refresh(force = false) {
    const key = context(); if (!key || !visible()) return true;
    if (activeMutation()) return false;
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
  window.homeProgressFor = id => snapshot?.key === context() ? snapshot.data.continue_watching.find(item => item.media.id === id)?.progress : null;
  window.clearHomeContext = () => { generation++; pending = snapshot = null; clearRails(); $("home-status").textContent = ""; $("home-empty").hidden = true; };
  window.homeVisibilityChanged = shown => window.catalogSetRecommendationRailVisible?.($("home-rails"), shown);
  window.refreshHomeAfterMutation = () => { if (visible()) void refresh(true); };
  document.addEventListener("visibilitychange", () => { if (visible()) void refresh(); });
})();
