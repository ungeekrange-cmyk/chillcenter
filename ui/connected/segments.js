"use strict";

(() => {
  const names = [["recap", "Récap"], ["intro", "Intro"]];
  const t = (key, ...values) => window.chillI18n.t(key, ...values);
  const capable = () => state.capabilities?.includes("playback_segments");
  const controller = $("player-stage");
  const skip = button("", () => {});
  skip.id = "player-skip-segment";
  skip.hidden = true;
  controller.append(skip);
  let request = null;

  function draw(result, position, duration) {
    const ms = position * 1000;
    const segment = result.find(item => Number.isSafeInteger(item.start_ms) && Number.isSafeInteger(item.end_ms)
      && item.start_ms <= ms && ms < item.end_ms && item.end_ms <= duration * 1000
      && (item.kind === "intro" || item.kind === "recap"));
    skip.hidden = !segment;
    if (!segment) return;
    skip.textContent = t(segment.kind === "intro" ? "Passer l’intro" : "Passer le récap");
    skip.onclick = () => void playerCall({ action: "seek", position: segment.end_ms / 1000 });
  }

  window.renderPlayerSegments = (player, epoch) => {
    skip.hidden = true;
    if (player.stage !== "player" || player.native?.ended || (!player.local && !capable())) {
      request = null;
      return;
    }
    const key = `${state.server_id}:${state.profile_id}:${epoch}:${player.media_id}:${Boolean(player.local)}`;
    if (request?.key === key) {
      if (request.result) draw(request.result, player.position || 0, player.duration || 0);
      return;
    }
    const pending = { key };
    request = pending;
    const target = { action: "segments", media_id: player.media_id };
    const query = player.local
      ? window.__TAURI__.core.invoke("downloads_command", { command: target })
      : invoke(target);
    void query.then(reply => {
      if (request !== pending) return;
      pending.result = Array.isArray(reply?.segments) ? reply.segments.slice(0, 2) : [];
      draw(pending.result, playerState.position || 0, playerState.duration || 0);
    }).catch(error => {
      if (request === pending) {
        // Competing duration probes can temporarily occupy the Pi's probe slot.
        if ([503, 504].includes(error?.status)) {
          setTimeout(() => { if (request === pending) request = null; }, 2000);
        } else {
          pending.result = [];
        }
      }
    });
  };

  window.segmentEditor = (media, scope, isCurrent) => {
    if (!capable() || media?.category !== "serie" || !media.available || (scope === "season" && media.season == null)) return null;
    const details = node("details", undefined, "segment-editor");
    details.append(node("summary", t("Repères d’intro et de récap")));
    const form = node("form");
    const status = node("p", "", "muted");
    status.setAttribute("role", "status");
    details.append(form, status);
    form.hidden = true;
    const current = () => details.isConnected && isCurrent();
    let saved;

    function paint() {
      form.replaceChildren();
      const rows = names.map(([kind, name]) => {
        const existing = saved.manual.find(segment => segment.kind === kind);
        const row = node("fieldset");
        row.append(node("legend", t(name)));
        const inputs = ["Début (s)", "Fin (s)"].map((label, index) => {
          const wrapper = node("label", t(label));
          const input = node("input");
          input.type = "number";
          input.step = "0.001";
          input.min = "0";
          if (saved.duration_ms) input.max = String(saved.duration_ms / 1000);
          input.value = existing ? String((index ? existing.end_ms : existing.start_ms) / 1000) : "";
          wrapper.append(input);
          row.append(wrapper);
          input.oninput = renderPreview;
          return input;
        });
        form.append(row);
        return { kind, inputs };
      });
      const track = node("div", undefined, "segment-preview");
      const preview = node("p", "", "muted");
      const save = node("button", t("Enregistrer les repères"), "primary");
      const reload = button(t("Recharger les repères"), () => void load());
      form.append(track, preview, save, reload);
      function values() {
        const entries = [];
        for (const { kind, inputs: [start, end] } of rows) {
          if (start.value === "" && end.value === "") continue;
          const from = Math.round(start.valueAsNumber * 1000);
          const to = Math.round(end.valueAsNumber * 1000);
          if (!Number.isSafeInteger(from) || !Number.isSafeInteger(to) || from < 0 || from >= to || !saved.duration_ms || to > saved.duration_ms) return null;
          entries.push({ kind, start_ms: from, end_ms: to });
        }
        if (entries.length === 2 && entries[0].start_ms < entries[1].end_ms && entries[1].start_ms < entries[0].end_ms) return null;
        return entries;
      }
      function renderPreview() {
        const entries = values();
        track.replaceChildren();
        track.hidden = !entries?.length;
        save.disabled = !entries;
        if (!entries) { preview.textContent = t("Vérifie le début et la fin de chaque repère."); return; }
        preview.textContent = entries.map(entry => t(entry.kind === "intro" ? "Intro" : "Récap")
          + " · " + timeLabel(entry.start_ms / 1000) + " – " + timeLabel(entry.end_ms / 1000)).join(" · ");
        for (const item of entries) {
          const band = node("span");
          band.dataset.kind = item.kind;
          band.style.left = 100 * item.start_ms / saved.duration_ms + "%";
          band.style.width = 100 * (item.end_ms - item.start_ms) / saved.duration_ms + "%";
          track.append(band);
        }
      }
      form.onsubmit = async event => {
        event.preventDefault();
        const segments = values();
        if (!segments || !current() || save.disabled) return;
        save.disabled = true;
        try {
          await invoke({ action: "save_segments", media_id: media.id, scope, version: saved.version, expected_revision: saved.revision, segments });
          if (current()) {
            await load();
            if (form.hidden === false) status.textContent = t("Repères enregistrés.");
          }
        } catch (error) {
          if (current()) status.textContent = error?.code === "PLAYBACK_SEGMENTS_CHANGED"
            ? t("Les repères ou le fichier ont changé. Recharge les repères.") : error?.message || t("Repères indisponibles.");
        } finally { save.disabled = false; }
      };
      renderPreview();
      form.hidden = false;
    }

    async function load() {
      status.textContent = t("Chargement…");
      form.hidden = true;
      try {
        const view = await invoke({ action: "segments", media_id: media.id, scope });
        if (!current()) return;
        saved = view;
        status.textContent = !view.duration_ms ? t("Durée indisponible pour cet épisode.") : "";
        paint();
      } catch (error) {
        if (current()) status.textContent = error?.message || t("Repères indisponibles.");
      }
    }
    details.ontoggle = () => { if (details.open && !saved) void load(); };
    return details;
  };
})();
