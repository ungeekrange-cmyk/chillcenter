"use strict";

(() => {
  const t = (key, ...args) => window.chillI18n.t(key, ...args);
  const web = Boolean(window.ChillBridge);
  const open = button(t("Ce soir à deux"), () => void show(), web ? "nav-item" : "secondary");
  open.id = "show-duo";
  open.hidden = true;
  if (web) {
    const icon = $("advice-toggle").querySelector("svg")?.cloneNode(true);
    if (icon) open.prepend(icon);
    $("advice-toggle").before(open);
  } else $("advice-panel").querySelector("h1").after(open);
  const dialog = node("dialog", undefined, "duo-dialog");
  dialog.setAttribute("aria-labelledby", "duo-title");
  const caption = node("h2", t("Ce soir à deux"));
  caption.id = "duo-title";
  const header = node("div", undefined, "section-head");
  header.append(caption, button(t("Fermer"), () => dialog.close()));
  const body = node("div");
  const feedback = node("p", "", "muted");
  feedback.setAttribute("role", "status");
  dialog.append(header, body, feedback);
  document.body.append(dialog);

  const playback = node("span", "", "player-duo");
  playback.hidden = true;
  $("player-title").after(playback);
  const scope = () => `${state.server_id}:${state.profile_id}`;
  const eligible = () => state.profile_id && !state.downloads_only && state.capabilities?.includes("duo_playback");
  const profileName = id => state.profiles?.find(profile => profile.id === id)?.name || "";
  let guest, step, generation = 0, intentScope, answers, questions, launching = false;

  function replace(...nodes) {
    for (const poster of body.querySelectorAll(".duo-poster")) window.retireArtwork?.(poster);
    body.replaceChildren(...nodes);
  }
  dialog.addEventListener("close", () => {
    if (!launching && !dialog.open) {
      generation++;
      replace();
    }
  });

  window.renderDuo = () => {
    open.hidden = !eligible() || !state.profiles?.some(profile => profile.id !== state.profile_id);
    if ((open.hidden || intentScope !== scope()) && dialog.open) dialog.close();
  };
  window.renderDuoPlayback = player => {
    const guestName = profileName(player.guest_profile_id);
    playback.hidden = player.stage !== "player" || !guestName;
    if (!playback.hidden) playback.textContent = t("À deux · {0} et {1}", profileName(state.profile_id), guestName);
  };

  async function show() {
    if (!eligible() || !await stopPlayerForNavigation()) return;
    intentScope = scope();
    profiles();
    if (!dialog.open) dialog.showModal();
  }

  function profiles() {
    generation++;
    feedback.textContent = "";
    const select = node("select");
    select.required = true;
    for (const profile of state.profiles.filter(profile => profile.id !== state.profile_id)) {
      const option = node("option", profile.name);
      option.value = profile.id;
      select.append(option);
    }
    const label = node("label", t("Avec qui regardes-tu ?"));
    label.append(select);
    replace(label, button(t("Surprends-moi"), () => {
      if (!select.value) return;
      guest = select.value;
      step = 0;
      answers = {};
      const key = "chillcenter-duo-flow:" + intentScope + ":" + guest;
      let previous = 0;
      try { previous = Number(sessionStorage.getItem(key)) || 0; } catch {}
      try { sessionStorage.setItem(key, String((previous + 1) % 3)); } catch {}
      questions = [["format", "discovery", "minutes"], ["discovery", "minutes", "mood"], ["mood", "format", "minutes"]][previous % 3];
      ask();
    }, "primary"));
  }

  function ask() {
    const key = questions[step];
    const definitions = {
      format: ["Film ou série ?", [["film", "Film"], ["series", "Série"], [null, "Les deux"]]],
      minutes: ["Combien de temps avez-vous ?", [[45, "45 minutes"], [90, "1 h 30"], [null, "Sans limite"]]],
      discovery: ["On retrouve quelque chose de familier ?", [["familiar", "Dans nos goûts"], ["different", "Surprends-nous"]]],
      mood: ["Quelle ambiance ?", [["light", "Quelque chose de léger"], ["intimate", "Quelque chose d’intime"], ["spectacle", "Un grand spectacle"]]],
    };
    const [question, choices] = definitions[key];
    const buttons = node("div", undefined, "actions");
    for (const [value, label] of choices) {
      buttons.append(button(t(label), () => {
        answers[key] = value;
        if (++step === questions.length) { void suggestions(); return; }
        // The next question follows a short budget with a format choice when available.
        if (key === "format" && value === "series" && questions[step] === "discovery") {
          questions[step] = "mood";
        } else if (key === "discovery" && value === "different" && questions.includes("mood", step)) {
          const index = questions.indexOf("mood", step);
          [questions[step], questions[index]] = [questions[index], questions[step]];
        }
        ask();
      }));
    }
    replace(node("h3", t(question)), buttons, button(t("Changer de profil"), profiles));
  }

  async function suggestions() {
    const currentId = ++generation;
    const current = () => generation === currentId && intentScope === scope();
    replace(node("p", t("Chargement…"), "muted"));
    const response = await libraryRead({ action: "duo_recommendations", query: { ...answers, guest_profile_id: guest } }, current);
    if (!current()) return;
    if (!response) {
      replace(button(t("Réessayer"), () => void suggestions()), button(t("Changer de profil"), profiles));
      return;
    }
    const grid = node("div", undefined, "duo-grid");
    const reasons = { mood: "Dans l’ambiance choisie", shared_genres: "Dans vos goûts communs", discovery: "À découvrir ensemble" };
    for (const item of response.items.slice(0, 9)) {
      const media = item.media;
      const series = media.category === "serie";
      const title = series ? media.series || media.title : media.title;
      const seriesKey = series ? (media.tmdb_id ? `tmdb:tv:${media.tmdb_id}` : `media:${media.id}`) : null;
      const tile = node("article");
      const poster = node("div", title.slice(0, 1), "poster duo-poster");
      const episode = series && media.episode != null ? t("Saison {0} · {1}", media.season ?? "?", `E${media.episode}`) : null;
      const detail = [episode, item.minutes == null ? t("Durée inconnue") : item.minutes + " min",
        item.watched_by_one ? t("Déjà vu par une personne") : null].filter(Boolean).join(" · ");
      const play = button(t("Regarder à deux"), () => {
        feedback.textContent = "";
        const position = item.position_ms > 0 ? timeLabel(item.position_ms / 1000) : t("Depuis le début");
        const actions = node("div", undefined, "actions");
        const confirmation = node("section");
        actions.append(button(t("Retour aux propositions"), () => confirmation.replaceWith(tile)), button(t("Lancer la lecture"), async () => {
          if (!current() || launching) return;
          launching = true;
          const slot = seriesKey ? { kind: "series", id: seriesKey } : null;
          try {
            const started = await window.chillPlay(media.id, slot, false, guest);
            if (!started && current() && eligible()) {
              feedback.textContent = playerLastError?.code === "DUO_PROFILE_BUSY"
                ? t("Un des deux profils utilise déjà un lecteur.") : t("La séance à deux n’a pas démarré.");
              if (!dialog.open) dialog.showModal();
            } else if (started) {
              replace();
            }
          } finally { launching = false; }
        }, "primary"));
        confirmation.append(node("h3", title), node("p", detail), node("p", t("À deux · {0} et {1}", profileName(state.profile_id), profileName(guest))),
          node("p", t("Départ · {0}", position)), actions);
        tile.replaceWith(confirmation);
      }, "primary");
      const details = button(t("Voir la fiche"), () => {
        dialog.close();
        if (seriesKey) void window.catalogOpenSeries(seriesKey);
        else void window.catalogOpenMedia(media.id);
      });
      tile.append(poster, node("h3", title), node("p", detail, "muted"),
        node("p", t(reasons[item.reason] || reasons.discovery), "muted"), details, play);
      grid.append(tile);
      if (media.poster) observeArtwork(poster, media.id);
    }
    const controls = node("div", undefined, "actions");
    controls.append(button(t("Changer de profil"), profiles), button(t("Recommencer"), profiles));
    replace(response.items.length ? grid : node("p", t("Aucune proposition ne correspond à vos choix."), "muted"), controls);
  }
  window.renderDuo();
})();
