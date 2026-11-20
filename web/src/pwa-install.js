"use strict";
/* Web-only PWA installation affordance.  The browser remains responsible for
 * acceptance: this code never says the app was installed before appinstalled. */
(function pwaInstall(root, doc) {
  const isFrench = () => String(doc.documentElement.lang || "").toLowerCase().startsWith("fr");
  const copy = () => isFrench() ? {
    install: "Installer l’app", close: "Fermer", unavailable: "L’installation nécessite HTTPS.",
    certificate: "Le navigateur ne peut pas vérifier le serveur. Ouvrez l’adresse HTTPS de Chill Center, vérifiez le certificat du serveur puis réessayez.",
    android: "Dans Chrome sur Android, ouvrez le menu ⋮ puis choisissez Installer l’application.",
    ios: "Dans Safari sur iPhone ou iPad, touchez Partager puis Ajouter à l’écran d’accueil.",
    macSafari: "Dans Safari sur Mac, choisissez Fichier puis Ajouter au Dock.",
    desktop: "Dans Chrome ou Vivaldi, ouvrez le menu puis choisissez Installer Chill Center.",
    title: "Installer Chill Center", installed: "Chill Center est installé sur cet appareil.",
  } : {
    install: "Install app", close: "Close", unavailable: "Installation requires HTTPS.",
    certificate: "The browser cannot verify the server. Open Chill Center over HTTPS, verify the server certificate, then try again.",
    android: "In Chrome on Android, open the ⋮ menu and choose Install app.",
    ios: "In Safari on iPhone or iPad, tap Share, then Add to Home Screen.",
    macSafari: "In Safari on Mac, choose File, then Add to Dock.",
    desktop: "In Chrome or Vivaldi, open the menu and choose Install Chill Center.",
    title: "Install Chill Center", installed: "Chill Center is installed on this device.",
  };
  const standalone = () => Boolean(root.navigator.standalone) || Boolean(root.matchMedia?.("(display-mode: standalone)").matches);
  const apple = () => /iPad|iPhone|iPod/.test(root.navigator.userAgent || "") || (root.navigator.platform === "MacIntel" && Number(root.navigator.maxTouchPoints) > 1);
  const safari = () => /Safari/.test(root.navigator.userAgent || "") && !/Chrome|Chromium|CriOS|Edg/.test(root.navigator.userAgent || "");
  const android = () => /Android/.test(root.navigator.userAgent || "");

  function helpText(reason) {
    if (!root.isSecureContext) return copy().unavailable;
    if (reason === "certificate-or-network") return copy().certificate;
    if (apple()) return copy().ios;
    if (safari()) return copy().macSafari;
    return android() ? copy().android : copy().desktop;
  }
  function create(tag, text) { const node = doc.createElement(tag); if (text) node.textContent = text; return node; }
  function install() {
    if (doc.getElementById("pwa-install")) return;
    const topbar = doc.querySelector(".app-topbar"); if (!topbar) return;
    const button = create("button", copy().install);
    button.id = "pwa-install"; button.type = "button"; button.className = "pwa-install secondary";
    button.setAttribute("aria-label", copy().install);
    button.hidden = standalone();
    const dialog = doc.createElement("dialog"); dialog.id = "pwa-install-dialog"; dialog.className = "pwa-install-dialog";
    const title = create("h2", copy().title), explanation = create("p"), close = create("button", copy().close);
    title.id = "pwa-install-title"; dialog.setAttribute("aria-labelledby", title.id);
    close.type = "button"; close.className = "secondary";
    dialog.append(title, explanation, close); doc.body.append(dialog); topbar.append(button);
    let deferredPrompt = null, unavailableReason = null;
    const hideIfStandalone = () => { button.hidden = standalone(); if (standalone() && dialog.open) dialog.close(); };
    const refreshLocale = () => {
      button.textContent = copy().install; button.setAttribute("aria-label", copy().install);
      title.textContent = copy().title; close.textContent = copy().close;
      if (dialog.open) explanation.textContent = helpText(unavailableReason);
    };
    const showHelp = () => { explanation.textContent = helpText(unavailableReason); if (!dialog.open) dialog.showModal(); close.focus(); };
    const closeHelp = () => { if (dialog.open) dialog.close(); button.focus(); };
    button.addEventListener("click", async () => {
      if (!deferredPrompt) return showHelp();
      const prompt = deferredPrompt; deferredPrompt = null;
      try { await prompt.prompt(); await prompt.userChoice; }
      catch { showHelp(); }
    });
    close.addEventListener("click", closeHelp);
    dialog.addEventListener("cancel", () => root.setTimeout(() => button.focus(), 0));
    dialog.addEventListener("click", event => { if (event.target === dialog) closeHelp(); });
    root.addEventListener("beforeinstallprompt", event => { event.preventDefault(); deferredPrompt = event; button.hidden = false; });
    root.addEventListener("appinstalled", () => { button.disabled = false; button.hidden = true; deferredPrompt = null; explanation.textContent = copy().installed; if (dialog.open) dialog.close(); });
    root.matchMedia?.("(display-mode: standalone)").addEventListener?.("change", hideIfStandalone);
    root.addEventListener("chill:pwa-status", event => {
      const status = event.detail || {};
      unavailableReason = status.state === "unavailable" ? status.reason || "certificate-or-network" : null;
      if (status.state === "unavailable" && !deferredPrompt && !standalone()) button.hidden = false;
    });
    doc.addEventListener("chill-language-change", refreshLocale);
    new MutationObserver(refreshLocale).observe(doc.documentElement, { attributes: true, attributeFilter: ["lang"] });
  }
  root.ChillPwaInstall = { standalone, helpText, install };
  if (doc.readyState === "loading") doc.addEventListener("DOMContentLoaded", install, { once: true }); else install();
})(window, document);
