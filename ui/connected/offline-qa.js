// Dev-only smoke against an isolated loopback server and synthetic media.
window.chillOfflineQa = async ({ address, mode }) => {
  const core = window.__TAURI__.core;
  const downloads = (action, fields = {}) => core.invoke("downloads_command", { command: { action, ...fields } });
  const finish = (success, detail) => core.invoke("qa_result", { success, detail });
  const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
  const require = (condition, detail) => { if (!condition) throw new Error(detail); };
  let phase = "boot";
  async function until(predicate, seconds = 20) {
    const deadline = Date.now() + seconds * 1000;
    do {
      const result = await predicate();
      if (result) return result;
      await sleep(120);
    } while (Date.now() < deadline);
    throw new Error(phase + " / " + $("message").textContent + " / " + $("player-note").textContent);
  }
  try {
    await finish(false, "TRACE " + mode + " startup");
    await window.chillReady;
    await finish(false, "TRACE " + mode + " boot finished");
    if (mode === "offline_prepare") {
      async function apply(command) {
        const view = await invoke(command);
        state = view;
        render();
      }
      phase = "connect";
      await apply({ action: "connect", address });
      require(state.stage === "trust", "loopback pin consent expected");
      phase = "approve certificate";
      await apply({ action: "approve", token: state.token });
      phase = "create profile";
      await apply({ action: "create_profile", name: "Offline QA" });
      phase = "select profile";
      require(state.profiles?.length, "profile missing");
      await apply({ action: "select_profile", id: state.profiles[0].id });
      require(state.profile_id && !state.downloads_only, JSON.stringify({ stage: state.stage, profile: state.profile_id, notice: state.notice }));
      phase = "find fixture";
      const media = await until(async () => {
        const catalog = await invoke({ action: "catalog", offset: 0, category: null, q: null });
        return catalog.items.find(item => item.title.includes("Offline Fixture"));
      });
      phase = "start download";
      await downloads("start", { media_id: media.id, scope: "video" });
      phase = "finish download";
      const ready = await until(async () => {
        const current = await downloads("list");
        const failed = current.jobs?.find(job => job.status.startsWith("error:"));
        require(!failed, failed?.status);
        return !current.busy && current.items[0] && current;
      });
      require(ready.enabled && ready.total > 0 && ready.free > 0 && ready.capacity >= ready.free, JSON.stringify(ready));
      require(ready.items[0].missing_subtitles === 0, "external subtitle unavailable");
      await finish(true, "Mac: HTTPS download, default policy and volume capacity");
      return;
    }
    phase = "cold reopen";
    require(state.downloads_only, "download-only state expected");
    const local = await downloads("list");
    await finish(false, "TRACE cold list items=" + local.items.length + " hidden=" + document.hidden);
    require(local.items.length === 1, "missing local movie");
    await window.refreshDownloads();
    require(!$("show-downloads").hidden, "download tab missing");
    phase = "native playback";
    state.player_presentation = { fullscreen_on_start: false };
    await finish(false, "TRACE before native launch");
    await window.chillPlay(local.items[0].media.id, null, true);
    await finish(false, "TRACE after native launch " + playerState.stage);
    clearInterval(playerTimer);
    const player = command => core.invoke("player_command", { command });
    const playing = await until(async () => {
      const snapshot = await player({ action: "status" });
      return snapshot.native?.loaded && snapshot.position > 0.5 && snapshot;
    });
    await finish(false, "TRACE native playback advanced");
    const subtitle = playing.external_subtitles?.[0];
    require(subtitle?.id, "local sidecar missing from player");
    const externalSelected = view => view.native?.tracks?.some(track =>
      track.kind === "Subtitle" && track.external && track.selected);
    if (!externalSelected(playing)) {
      phase = "select external subtitle";
      await player({ action: "external_subtitle", id: subtitle.id });
      await until(async () => externalSelected(await player({ action: "status" })), 6);
    }
    // Python checks EOF and the disk independently of WebKit's background timer throttling.
    await finish(false, "TRACE cold playback with external subtitle selected");
  } catch (error) {
    await finish(false, phase + ": " + (error?.message || JSON.stringify(error)));
  }
};
