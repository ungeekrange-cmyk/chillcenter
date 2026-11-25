"use strict";
(() => {
  const kinds = [
    ["duplicates", "Doublons probables"],
    ["missing_episodes", "Épisodes manquants"],
    ["unavailable", "Vidéos indisponibles"],
    ["single_title_sagas", "Sagas à un seul titre"],
  ];
  const t = (key, ...args) => window.chillI18n.t(key, ...args);
  let section, choices, status, results, kind = kinds[0][0], generation = 0;

  const context = () => `${state.server_id}:${state.profile_id}`;
  const active = () => Boolean(state.profile_id && !state.downloads_only && state.capabilities?.includes("library_audit"));

  function mediaLabel(item) {
    return button(item.title, () => window.catalogOpenMedia(item.id));
  }

  function showIssue(issue, refresh) {
    const row = node("li");
    if (issue.kind === "duplicate") {
      row.append(node("p", t("Copies signalées : {0}", issue.count)));
      for (const item of issue.media) {
        const entry = node("div", undefined, "audit-media");
        entry.append(mediaLabel(item), node("small", item.relative_path, "muted"));
        row.append(entry);
      }
    } else if (issue.kind === "missing") {
      row.append(button(issue.series, () => window.catalogOpenMedia(issue.anchor_id)));
      const number = issue.from === issue.through ? `E${issue.from}` : `E${issue.from}–E${issue.through}`;
      row.append(node("p", t("Saison {0} · {1}", issue.season, number), "muted"));
    } else if (issue.kind === "saga") {
      row.append(node("p", issue.name), mediaLabel(issue.member));
      row.append(button(t("Retirer la saga"), async () => {
        if (!confirm(t("Retirer la saga « {0} » ? « {1} » et son historique restent au catalogue.", issue.name, issue.member.title))) return;
        await libraryRead({ action: "delete_saga", id: issue.id, expected_revision: issue.revision }, refresh.current);
        if (refresh.current()) {
          await refresh.reload();
          void syncAfterMutation();
        }
      }, "danger"));
    }
    return row;
  }

  async function load(offset = 0) {
    const request = ++generation, scope = context();
    const current = () => request === generation && shellSection === "admin" && active() && scope === context();
    const selectedKind = kind;
    results.replaceChildren(node("p", t("Chargement…"), "muted"));
    status.textContent = "";
    const page = await libraryRead({ action: "library_audit", kind, offset }, current);
    if (!current()) return;
    if (!page) {
      results.replaceChildren(button(t("Réessayer"), () => load(offset)));
      return;
    }
    status.textContent = page.last_scan == null
      ? t("Aucun scan terminé.")
      : t("Dernier scan terminé : {0}", new Date(page.last_scan * 1000).toLocaleString());
    if (page.last_scan_failed) {
      status.append(document.createTextNode(" " + (page.failed_root
        ? t("Dernier essai incomplet sur l’emplacement {0}.", page.failed_root)
        : t("Dernier essai de scan incomplet."))));
    }
    const refresh = { current, reload: () => load(offset) };
    const issues = node("ul", undefined, "audit-issues");
    const limit = selectedKind === "unavailable" ? 48 : 24;
    for (const item of page.items) {
      if (selectedKind === "unavailable") {
        const row = node("li");
        row.append(mediaLabel(item), node("p", item.relative_path, "muted"));
        row.append(button(t("Retirer la référence"), async () => {
          if (!confirm(t("Retirer « {0} » du catalogue ? Reprises, notes et états vus seront effacés pour tous les profils. Le fichier source reste intact.", item.title))) return;
          await libraryRead({ action: "forget_unavailable", media_ids: [item.id], expected_epoch: page.epoch, confirmed: true }, current);
          if (current()) {
            await load(offset);
            void syncAfterMutation();
          }
        }, "danger"));
        issues.append(row);
      } else {
        issues.append(showIssue(item, refresh));
      }
    }
    if (issues.children.length) results.replaceChildren(issues);
    else results.replaceChildren(node("p", t("Aucun signalement dans cette vue."), "muted"));
    if (offset || page.has_more) {
      const nav = node("div", undefined, "actions");
      const previous = button(t("Précédent"), () => load(Math.max(0, offset - limit)));
      previous.disabled = offset === 0;
      nav.append(previous);
      if (page.has_more) nav.append(button(t("Suivant"), () => load(offset + limit)));
      results.append(nav);
    }
  }

  window.openLibraryAudit = () => {
    if (!active()) {
      generation++;
      section?.remove();
      section = null;
      return false;
    }
    if (!section) {
      section = node("section");
      section.id = "library-audit";
      section.dataset.settingsCategory = "library";
      choices = node("select");
      for (const [value, label] of kinds) {
        const option = node("option", t(label));
        option.value = value;
        choices.append(option);
      }
      choices.value = kind;
      choices.onchange = () => { kind = choices.value; void load(); };
      const label = node("label", t("Signalements"));
      label.append(choices);
      status = node("p", "", "muted");
      status.setAttribute("role", "status");
      results = node("div");
      section.append(node("h2", t("Bilan de la bibliothèque")), label, status, results);
      $("admin-panel").querySelector(".admin-grid").append(section);
    }
    $("unavailable-admin")?.remove();
    void load();
    return true;
  };
})();
