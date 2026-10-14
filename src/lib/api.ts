// Rust çekirdeğine giden tüm çağrılar burada. Uygulama Tauri dışında (tarayıcıda `npm run dev`)
// açıldığında da çalışsın diye her çağrının küçük bir tarayıcı karşılığı var.
import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export interface BackupInfo {
  name: string;
  size: number;
  created: string;
}

export interface AppInfo {
  version: string;
  dataDir: string;
  dbSize: number;
  backups: BackupInfo[];
}

const BG_EXTENSIONS = ["jpg", "jpeg", "png", "webp", "gif", "mp4", "webm"];

function browserStore(): Record<string, string> {
  try {
    return JSON.parse(localStorage.getItem("gmid.settings") ?? "{}");
  } catch {
    return {};
  }
}

import type {
  FootballGroup,
  FootballLeaders,
  FootballMatch,
  FootballTeamPage,
  CleanResult, InstalledProgram,
  ProgramList, LeftoverScan, UninstallState,
  KaraokeHit, KaraokeMix, KaraokeRecording, KaraokeStatus,
  Alerts, AppVolume, AudioState, BackupManifest, DealsSnapshot, FpsStatus, ServerWatch, PerfSample, SearchHit, Wrapped, FitOptions, FitPlan, FitResult, FitChoice, UpdateStatus, InstallHit, InstallItem, InstallRun, KitRequest, KitResult, BridgeStatus, DictationStatus, DictRule, PowerPlan, Specs, NetReport, DriverStatus, NowPlaying, Clip, ClipRoot, FoundGame, GameModeStatus, GameSuggestion, Title, TitleHit, Game, PlaySession, RunningApp, Review, ReviewInput, StoreHit, Download, Langs, Segment, Sidecar, UserPlaylist, GameInfo, Live, Parsed, ToolsState, Transcript, WhisperStatus, HotkeyStatus, JoinResult, MicStatus, Note, NoteInput, NoteSummary, OsdMessage,
  Reminder, ReminderInput, Server, ServerInput, ServerStatus, SyncResult, TimersSnapshot,
} from "./types";

/** Bir Rust komutunu çağırır; tarayıcıda taklit katmana gider. */
export async function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (inTauri) return invoke<T>(cmd, args);
  const mock = await import("./mock");
  return mock.handle(cmd, args) as Promise<T>;
}

export const api = {
  getSettings: (): Promise<Record<string, string>> =>
    inTauri ? invoke("get_settings") : Promise.resolve(browserStore()),

  setSetting: async (key: string, value: string): Promise<void> => {
    if (inTauri) return invoke("set_setting", { key, value });
    try {
      localStorage.setItem("gmid.settings", JSON.stringify({ ...browserStore(), [key]: value }));
    } catch {
      /* tarayıcı depolaması kapalıysa ayar sadece bu oturumda kalır */
    }
  },

  takeSplash: (): Promise<boolean> => (inTauri ? invoke("take_splash") : Promise.resolve(true)),

  takePendingRoute: (): Promise<string | null> =>
    inTauri ? invoke("take_pending_route") : Promise.resolve(null),

  appInfo: (): Promise<AppInfo> =>
    inTauri
      ? invoke("app_info")
      : Promise.resolve({
          version: "0.1.0",
          dataDir: "C:\\Users\\Sen\\Documents\\GMID Tool",
          dbSize: 24576,
          backups: [{ name: "gmid-2026-10-11_09-00-12.db", size: 24576, created: "2026-10-11 09:00" }],
        }),

  openDataFolder: (sub?: string): Promise<void> =>
    inTauri ? invoke("open_data_folder", { sub: sub ?? null }) : Promise.resolve(),

  football: {
    leagues: () => call<[string, string][]>("football_leagues"),
    /** Günler: "YYYYMMDD". */
    matches: (leagues: string[], days: string[]) => call<FootballMatch[]>("football_matches", { leagues, days }),
    /** İki tarih (YYYYMMDD) arası, en çok 10 gün. */
    week: (leagues: string[], from: string, to: string) => call<FootballMatch[]>("football_week", { leagues, from, to }),
    standings: (league: string) => call<FootballGroup[]>("football_standings", { league }),
    team: (league: string, id: string) => call<FootballTeamPage>("football_team", { league, id }),
    leaders: (league: string) => call<FootballLeaders>("football_leaders", { league }),
    /** Favorilerin canlı / bugünkü maçları; `show`: panelde gösterilsin mi. */
    live: () => call<{ show: boolean; matches: FootballMatch[] }>("football_live"),
    /** Favoriler değişti: hemen bak. */
    refreshLive: () => call<void>("football_refresh_live"),
  },

  /** Dosyaları uygulamadan dışarı sürükler (fare basılıyken çağrılır); bırakılan yere kopyalanır. */
  dragOut: (paths: string[]): Promise<void> => (inTauri ? invoke("drag_out", { paths }) : Promise.resolve()),

  /** Günlük dosyasını (gmid.log) açar. */
  openLog: (): Promise<void> => (inTauri ? invoke("open_log") : Promise.resolve()),

  backupNow: (): Promise<BackupInfo[]> => (inTauri ? invoke("backup_now") : api.appInfo().then((i) => i.backups)),

  /** Dosya seçtirir ve `Arkaplanlar` klasörüne kopyalar. Vazgeçilirse `null`. */
  pickBackground: async (): Promise<string | null> => {
    if (!inTauri) return null;
    const { open } = await import("@tauri-apps/plugin-dialog");
    const picked = await open({
      multiple: false,
      directory: false,
      title: "Arka plan seç",
      filters: [{ name: "Resim, GIF veya video", extensions: BG_EXTENSIONS }],
    });
    if (typeof picked !== "string") return null;
    return invoke<string>("import_background", { source: picked });
  },

  autostart: {
    isEnabled: async (): Promise<boolean> => {
      if (!inTauri) return false;
      const m = await import("@tauri-apps/plugin-autostart");
      return m.isEnabled();
    },
    set: async (on: boolean): Promise<void> => {
      if (!inTauri) return;
      const m = await import("@tauri-apps/plugin-autostart");
      await (on ? m.enable() : m.disable());
    },
  },

  /** Diskteki bir dosyayı arayüzde gösterilebilir adrese çevirir. */
  fileSrc: (path: string): string => (inTauri ? convertFileSrc(path) : path),

  onNavigate: async (cb: (route: string) => void): Promise<() => void> => {
    if (!inTauri) return () => {};
    const { listen } = await import("@tauri-apps/api/event");
    return listen<string>("navigate", (e) => cb(e.payload));
  },

  audio: {
    state: () => call<AudioState>("audio_state"),
    setDefault: (id: string) => call<void>("audio_set_default", { id }),
    setVolume: (id: string, level: number) => call<void>("audio_set_volume", { id, level }),
    setMute: (id: string, muted: boolean) => call<void>("audio_set_mute", { id, muted }),
    /** `null` kilidi kapatır. */
    setLock: (level: number | null) => call<MicStatus>("mic_set_lock", { level }),
    /** Uygulama başına ses (varsayılan çıkış). */
    apps: () => call<AppVolume[]>("audio_apps"),
    setAppVolume: (pid: number, level: number) => call<void>("audio_set_app_volume", { pid, level }),
    setAppMute: (pid: number, muted: boolean) => call<void>("audio_set_app_mute", { pid, muted }),
  },

  hotkeys: {
    list: () => call<HotkeyStatus[]>("hotkeys_list"),
    /** `null` varsayılana döndürür, `""` kapatır. */
    set: (id: string, accel: string | null) => call<HotkeyStatus[]>("hotkey_set", { id, accel }),
  },

  notes: {
    list: (trash = false) => call<NoteSummary[]>("notes_list", { trash }),
    get: (id: string) => call<Note | null>("note_get", { id }),
    save: (input: NoteInput) => call<Note>("note_save", { input }),
    trash: (id: string) => call<void>("note_trash", { id }),
    restore: (id: string) => call<void>("note_restore", { id }),
    remove: (id: string) => call<void>("note_delete", { id }),
    openQuick: () => call<void>("open_quick_note"),
    openSticky: (id: string) => call<void>("open_sticky", { id }),
    closeSticky: (id: string) => call<void>("close_sticky", { id }),
  },

  reminders: {
    list: () => call<Reminder[]>("reminders_list"),
    save: (input: ReminderInput) => call<Reminder>("reminder_save", { input }),
    remove: (id: string) => call<void>("reminder_delete", { id }),
    complete: (id: string) => call<void>("reminder_complete", { id }),
    snooze: (id: string, minutes: number) => call<void>("reminder_snooze", { id, minutes }),
  },

  timers: {
    state: () => call<TimersSnapshot>("timers_state"),
    stopwatchToggle: () => call<TimersSnapshot>("stopwatch_toggle"),
    stopwatchLap: () => call<TimersSnapshot>("stopwatch_lap"),
    stopwatchReset: () => call<TimersSnapshot>("stopwatch_reset"),
    add: (label: string, ms: number) => call<TimersSnapshot>("countdown_add", { label, ms }),
    toggle: (id: string) => call<TimersSnapshot>("countdown_toggle", { id }),
    remove: (id: string) => call<TimersSnapshot>("countdown_remove", { id }),
  },

  servers: {
    games: () => call<GameInfo[]>("servers_games"),
    list: () => call<Server[]>("servers_list"),
    /** Topluluk listesini (gerekiyorsa) indirir; `force` ile hemen. */
    sync: (force = false) => call<SyncResult>("servers_sync", { force }),
    cached: () => call<ServerStatus[]>("servers_cached"),
    /** Boş liste: hepsi. */
    query: (ids: string[] = []) => call<ServerStatus[]>("servers_query", { ids }),
    save: (input: ServerInput) => call<Server>("server_save", { input }),
    remove: (id: string) => call<void>("server_delete", { id }),
    join: (id: string) => call<JoinResult>("server_join", { id }),
    setExe: (game: string, path: string) => call<void>("server_set_exe", { game, path }),
    /** Logo seçtirir ve `Sunucular` klasörüne kopyalar. Vazgeçilirse `null`. */
    pickLogo: async (): Promise<string | null> => {
      if (!inTauri) return null;
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({
        multiple: false,
        directory: false,
        title: "Sunucu logosu seç",
        filters: [{ name: "Resim", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
      });
      if (typeof picked !== "string") return null;
      return invoke<string>("server_import_logo", { source: picked });
    },
    /** Oyunun .exe dosyasını seçtirir. */
    pickExe: async (gameName: string, exe: string): Promise<string | null> => {
      if (!inTauri) return null;
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({
        multiple: false,
        directory: false,
        title: `${gameName} · ${exe} dosyasını seç`,
        filters: [{ name: exe, extensions: ["exe"] }],
      });
      return typeof picked === "string" ? picked : null;
    },
  },

  videos: {
    /** Küçük resim açılamadıysa onarır (siteden yeniden indirir ya da videodan kare çıkarır). */
    repairThumb: (id: string) => call<boolean>("videos_repair_thumb", { id }),
    tools: () => call<ToolsState>("videos_tools"),
    installTools: (force = false) => call<ToolsState>("videos_install_tools", { force }),
    update: (force = false) => call<ToolsState>("videos_update", { force }),
    /** `wholeList`: video bağlantısındaki çalma listesinin tamamını getir. */
    probe: (url: string, wholeList = false) => call<Parsed>("videos_probe", { url, wholeList }),
    /** `ids` boşsa listenin tamamı. Eklenen video sayısını döner. */
    downloadPlaylist: (url: string, option: string, ids: string[] | null) =>
      call<number>("videos_download_playlist", { url, option, ids }),
    download: (url: string, option: string, langs: Langs | null = null) =>
      call<Download>("videos_download", { url, option, langs }),
    playlists: () => call<UserPlaylist[]>("playlists_list"),
    /** `id` boşsa yeni liste. */
    savePlaylist: (id: string | null, name: string, items: string[]) =>
      call<UserPlaylist>("playlist_save", { id, name, items }),
    deletePlaylist: (id: string) => call<void>("playlist_delete", { id }),
    /** Videonun yanındaki altyazılar ve seçilenin satırları (oynatıcı için). */
    tracks: (id: string) => call<Sidecar[]>("media_tracks", { id }),
    track: (id: string, file: string) => call<Segment[]>("media_track", { id, file }),
    list: () => call<Download[]>("videos_list"),
    live: () => call<Live[]>("videos_live"),
    pause: (id: string) => call<void>("videos_pause", { id }),
    resume: (id: string) => call<void>("videos_resume", { id }),
    cancel: (id: string) => call<void>("videos_cancel", { id }),
    remove: (id: string, deleteFile: boolean) => call<void>("videos_remove", { id, deleteFile }),
    open: (id: string) => call<void>("videos_open", { id }),
    reveal: (id: string) => call<void>("videos_reveal", { id }),
    bridge: () => call<BridgeStatus>("bridge_status"),
    pair: () => call<string>("bridge_pair"),
    unpair: () => call<void>("bridge_unpair"),
    installExtension: () => call<string>("extension_install"),
  },

  games: {
    list: () => call<Game[]>("games_list"),
    /** Steam ve Epic'te kurulu oyunlar (eklemez). */
    scan: () => call<FoundGame[]>("games_scan"),
    import: (found: FoundGame[]) => call<number>("games_import", { found }),
    addManual: (name: string, exe: string) => call<void>("games_add_manual", { name, exe }),
    runningApps: () => call<RunningApp[]>("games_running_apps"),
    setHidden: (id: string, hidden: boolean) => call<void>("games_set_hidden", { id, hidden }),
    rename: (id: string, name: string) => call<void>("games_rename", { id, name }),
    remove: (id: string) => call<void>("games_remove", { id }),
    refreshMeta: (id: string) => call<void>("games_refresh_meta", { id }),
    sessions: (since: number) => call<PlaySession[]>("games_sessions", { since }),
    launch: (id: string) => call<void>("games_launch", { id }),
    reveal: (id: string) => call<void>("games_reveal", { id }),
    /** Oyunun .exe dosyasını seçtirir. */
    pickExe: async (): Promise<string | null> => {
      if (!inTauri) return "C:\\Oyunlar\\Deneme\\Deneme.exe";
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({
        multiple: false,
        directory: false,
        title: "Oyunun .exe dosyasını seç",
        filters: [{ name: "Oyun", extensions: ["exe"] }],
      });
      return typeof picked === "string" ? picked : null;
    },
    /** Kütüphanede olmayan, açıldığı görülen oyunlar. */
    suggestions: () => call<GameSuggestion[]>("games_suggestions"),
    /** add: kütüphaneye ekle · later: şimdi değil · never: bir daha sorma. */
    answerSuggestion: (path: string, action: "add" | "later" | "never", name?: string) =>
      call<void>("games_suggestion_answer", { path, action, name: name ?? null }),
    /** Kapak resmi seçtirip oyuna koyar (Steam'de olmayan oyunlar için). */
    pickCover: async (id: string): Promise<boolean> => {
      if (!inTauri) return false;
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({
        multiple: false,
        directory: false,
        title: "Kapak resmi seç (dikey resimler en iyi görünür)",
        filters: [{ name: "Resim", extensions: ["jpg", "jpeg", "png", "webp", "gif"] }],
      });
      if (typeof picked !== "string") return false;
      await call<void>("games_set_cover", { id, source: picked });
      return true;
    },
    /** Steam mağazasında adıyla arama (Oyun Arşivi'ne ekleme). */
    storeSearch: (term: string) => call<StoreHit[]>("games_store_search", { term }),
    /** Arşive oyun ekler; oyunun kimliği. */
    addArchive: (appid: number | null, name: string) => call<string>("games_add_archive", { appid, name }),
  },

  clips: {
    list: () => call<Clip[]>("clips_list"),
    roots: () => call<ClipRoot[]>("clips_roots"),
    rescan: () => call<void>("clips_rescan"),
    setFavorite: (id: string, on: boolean) => call<void>("clips_set_favorite", { id, on }),
    rename: (id: string, title: string) => call<void>("clips_rename", { id, title }),
    setGame: (id: string, game: string | null) => call<void>("clips_set_game", { id, game }),
    /** Geri Dönüşüm Kutusu'na. */
    remove: (id: string) => call<void>("clips_delete", { id }),
    open: (id: string) => call<void>("clips_open", { id }),
    reveal: (id: string) => call<void>("clips_reveal", { id }),
    /** Seçilen boyuta sığan (istersen kırpılmış) kopya panoya; kopyanın yolu. Seçenek yoksa kayıtlı varsayılanlar. */
    fitDiscord: (id: string, options?: FitOptions) => call<FitResult>("clips_fit_discord", { id, options: options ?? null }),
    /** "Discord'a hazırla" seçenekleri (seçime göre). */
    choices: (id: string, start: number | null, end: number | null) => call<FitChoice[]>("clips_choices", { id, start, end }),
    /** Hazırlanan kopyayı yeniden panoya koyar. */
    readyCopy: (path: string) => call<void>("clips_ready_copy", { path }),
    readyReveal: (path: string) => call<void>("clips_ready_reveal", { path }),
    /** Seçime göre beklenen çözünürlük ve netlik. */
    plan: (id: string, options: FitOptions) => call<FitPlan>("clips_plan", { id, options }),
    /** İzlenecek klasör seçtirir. */
    pickFolder: async (): Promise<string | null> => {
      if (!inTauri) return "D:\\Kayıtlar";
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({ multiple: false, directory: true, title: "Klip klasörünü seç" });
      return typeof picked === "string" ? picked : null;
    },
  },

  serverWatch: {
    rules: () => call<Record<string, ServerWatch>>("servers_watch_rules"),
    /** Kapalı kural (ikisi de boş) kaldırılır. */
    set: (id: string, rule: ServerWatch) => call<void>("servers_watch_set", { id, rule }),
  },

  report: {
    /** Sorun raporu (zip) oluşturur, klasörde gösterir; panoya konacak özeti döner. */
    create: (description: string, withLogs: boolean) =>
      call<{ path: string; summary: string }>("report_create", { description, withLogs }),
  },

  community: {
    /** GMID Discord davet bağlantısı (yoksa null). */
    discord: () => call<string | null>("community_discord"),
  },

  borderless: {
    /** Kenarlıksız tam ekrana çevrilen oyunlar (exe adları). */
    list: () => call<string[]>("borderless_list"),
    set: (exe: string, on: boolean) => call<string[]>("borderless_set", { exe, on }),
  },

  fps: {
    status: () => call<FpsStatus>("fps_status"),
    /** PresentMon'u indirir, bir kez yönetici izniyle FPS ölçümüne izin verir. */
    setup: () => call<FpsStatus>("fps_setup"),
  },

  gamemode: {
    status: () => call<GameModeStatus>("gamemode_status"),
    /** Açar/kapatır; kapatınca değiştirilenler geri alınır. */
    set: (on: boolean) => call<GameModeStatus>("gamemode_set", { on }),
  },

  panel: {
    /** Oyun İçi Panel'i kapatır, odak oyuna döner. */
    hide: () => call<void>("panel_hide"),
    /** Nota yazmak için klavye odağı (panel normalde odak almaz, oyun önde kalır). */
    focus: () => call<void>("panel_focus"),
    /** Bakış kipinden etkileşime: panel odak alır, oyun fareyi bırakır. */
    interact: () => call<void>("panel_interact"),
    /** FPS (RTSS açıksa), işlemci, RAM, ekran kartı. */
    perf: () => call<PerfSample>("panel_perf"),
  },

  /** Bağlantıyı varsayılan tarayıcıda açar (yalnız https). */
  openUrl: (url: string) => (inTauri ? invoke<void>("open_url", { url }) : (window.open(url, "_blank"), Promise.resolve())),

  /** Notlar, oyunlar, film/diziler, videolar, klipler, hatırlatıcılar. */
  search: (query: string) => call<SearchHit[]>("search_all", { query }),

  wrapped: {
    get: (from: number, to: number) => call<Wrapped>("wrapped_get", { from, to }),
    /** PNG'yi (data: adresi) Ozetler klasörüne kaydeder, klasörde gösterir; yolu döner. */
    save: (name: string, png: string) => call<string>("wrapped_save", { name, png }),
  },

  deals: {
    get: () => call<DealsSnapshot>("deals_get"),
    /** Kaynaklara şimdi bakar (birkaç saniye sürebilir). */
    refresh: () => call<DealsSnapshot>("deals_refresh"),
  },

  update: {
    status: () => call<UpdateStatus>("update_status"),
    /** GitHub'daki en son sürüme bakar. */
    check: () => call<UpdateStatus>("update_check"),
    /** İndirir, doğrular, kurar; uygulama kapanıp yeni sürümle açılır. İlerleme "update" olayıyla. */
    install: () => call<void>("update_install"),
  },

  install: {
    catalog: () => call<InstallItem[]>("install_catalog"),
    list: () => call<InstallItem[]>("install_list"),
    saveList: (items: InstallItem[]) => call<void>("install_save_list", { items }),
    state: () => call<InstallRun>("install_state"),
    /** Sırayla sessiz kurulum; ilerleme "install" olayıyla. */
    run: (items: InstallItem[]) => call<InstallRun>("install_run", { items }),
    /** Şu an kurulan bitince durur. */
    stop: () => call<void>("install_stop"),
    /** Kurulu programların winget kimlikleri (küçük harf). */
    installed: (refresh = false) => call<string[]>("install_installed", { refresh }),
    search: (query: string) => call<InstallHit[]>("install_search", { query }),
    /** GMID'in kurulum dosyası İndirilenler/Masaüstü'nde varsa yolu. */
    findSetup: () => call<string | null>("install_find_setup"),
    backupSize: (options: { videos: boolean; tools: boolean }) => call<number>("install_backup_size", { options }),
    /** Format Kiti'ni oluşturur; ilerleme "install-kit" olayıyla. */
    exportKit: (request: KitRequest) => call<KitResult>("install_export", { request }),
    openKit: (path: string) => call<void>("install_open_kit", { path }),
    pickFolder: async (): Promise<string | null> => {
      if (!inTauri) return "E:\\";
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({ directory: true, multiple: false, title: "Format Kiti nereye konsun? (ör. USB bellek)" });
      return typeof picked === "string" ? picked : null;
    },
    pickSetup: async (): Promise<string | null> => {
      if (!inTauri) return "C:\\Users\\Sen\\Downloads\\GMID Tool_0.1.0_x64-setup.exe";
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({ multiple: false, title: "GMID Tool kurulum dosyası", filters: [{ name: "Kurulum dosyası", extensions: ["exe"] }] });
      return typeof picked === "string" ? picked : null;
    },
  },

  restore: {
    pick: async (): Promise<string | null> => {
      if (!inTauri) return "E:\\GMID Format Kiti\\GMID-Yedek.zip";
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({ multiple: false, title: "GMID yedeğini seç", filters: [{ name: "GMID yedeği", extensions: ["zip"] }] });
      return typeof picked === "string" ? picked : null;
    },
    /** Yedeği doğrular ve hazırlar; henüz hiçbir şey değişmez. */
    stage: (path: string) => call<BackupManifest>("backup_restore_stage", { path }),
    cancel: () => call<void>("backup_restore_cancel"),
    /** Uygulama yeniden başlar ve yedek yerine konur. */
    apply: () => call<void>("backup_restore_apply"),
  },

  titles: {
    list: () => call<Title[]>("titles_list"),
    /** Wikipedia'da film/dizi arar. */
    search: (term: string) => call<TitleHit[]>("titles_search", { term }),
    /** Arama sonucundan ya da yalnızca adıyla ekler. */
    add: (hit: TitleHit | null, name?: string, kind?: "movie" | "series") =>
      call<Title>("titles_add", { hit, name: name ?? null, kind: kind ?? null }),
    /** Puanlanmadan vazgeçildiyse geri alır. */
    discard: (id: string) => call<void>("titles_discard", { id }),
    refresh: (id: string) => call<void>("titles_refresh", { id }),
    /** Afişi elle seçtirir; vazgeçilirse `false`. */
    pickPoster: async (id: string): Promise<boolean> => {
      if (!inTauri) return false;
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({
        multiple: false,
        directory: false,
        title: "Afiş seç (dikey resimler en iyi görünür)",
        filters: [{ name: "Resim", extensions: ["jpg", "jpeg", "png", "webp", "gif"] }],
      });
      if (typeof picked !== "string") return false;
      await invoke("titles_set_poster", { id, source: picked });
      return true;
    },
  },

  reviews: {
    list: (kind: "game" | "title" = "game") => call<Review[]>("reviews_list", { kind }),
    save: (input: ReviewInput) => call<Review>("review_save", { input }),
    remove: (id: string) => call<void>("review_delete", { id }),
  },

  dictation: {
    status: () => call<DictationStatus>("dictation_status"),
    /** Ayarlar kaydedildikten sonra: açar/kapatır, sunucular yeni ayarla başlar. */
    apply: () => call<DictationStatus>("dictation_apply"),
    /** `part`: "whisper" | "gpu" (NVIDIA hızlandırma) | "llm" (toparlayıcı) */
    install: (part: "whisper" | "gpu" | "llm") => call<DictationStatus>("dictation_install", { part }),
    warmUp: () => call<DictationStatus>("dictation_warm_up"),
    /** Öğrenen sözlük: "şunu duyunca bunu yaz". */
    rules: () => call<DictRule[]>("dictation_rules"),
    ruleAdd: (from: string, to: string) => call<DictRule[]>("dictation_rule_add", { from, to }),
    ruleRemove: (from: string) => call<DictRule[]>("dictation_rule_remove", { from }),
    /** "Bunu böyle mi yazayım?" kartına yanıt. */
    learn: (id: number, accept: boolean) => call<void>("dictation_learn", { id, accept }),
    /** Son diktelerden birini düzelt: öğrenilecek [eski, yeni] çiftleri. */
    correct: (at: number, text: string) => call<[string, string][]>("dictation_correct", { at, text }),
  },

  uninstall: {
    list: () => call<ProgramList>("uninstall_list"),
    openFolder: (id: string) => call<void>("uninstall_open_folder", { id }),
    /** Program simgeleri: kimlik → PNG (data URL). */
    icons: () => call<Record<string, string>>("uninstall_icons"),
    /** Programın kendi kaldırıcısını açar. */
    run: (id: string) => call<void>("uninstall_run", { id }),
    state: (id: string) => call<UninstallState>("uninstall_state", { id }),
    scan: (id: string) => call<LeftoverScan>("uninstall_scan", { id }),
    /** `keys`: "tür|yol|değer" */
    clean: (id: string, keys: string[]) => call<CleanResult>("uninstall_clean", { id, keys }),
  },

  karaoke: {
    status: (id: string) => call<KaraokeStatus>("karaoke_status", { id }),
    /** `cpu`: ekran kartını kullanma. */
    prepare: (id: string, cpu = false) => call<KaraokeStatus>("karaoke_prepare", { id, cpu }),
    cancel: () => call<void>("karaoke_cancel"),
    /** YouTube'da temiz (instrumental / karaoke) sürüm arar. */
    search: (query: string) => call<KaraokeHit[]>("karaoke_search", { query }),
    /** Seçilen sürümü indirip şarkıya hizalar. */
    online: (id: string, url: string, title: string) => call<KaraokeStatus>("karaoke_online", { id, url, title }),
    onlineRemove: (id: string) => call<KaraokeStatus>("karaoke_online_remove", { id }),
    /** Mikrofon kaydını (base64) saklar; yolunu döner. */
    voice: (data: string, ext: string) => call<string>("karaoke_voice", { data, ext }),
    mix: (id: string, mix: KaraokeMix) => call<string>("karaoke_mix", { id, mix }),
    recordings: () => call<KaraokeRecording[]>("karaoke_recordings"),
    reveal: (path: string) => call<void>("karaoke_reveal", { path }),
    remove: (path: string) => call<void>("karaoke_delete", { path }),
  },

  transcripts: {
    get: (id: string) => call<Transcript | null>("transcript_get", { id }),
    /** `mode`: "auto" (önce sitenin altyazısı) | "subtitles" | "whisper"; `lang`: "orig" | "tr" | "en" */
    create: (id: string, mode: string, lang: string) => call<Transcript>("transcript_create", { id, mode, lang }),
    remove: (id: string) => call<void>("transcript_delete", { id }),
    whisperStatus: () => call<WhisperStatus>("whisper_status"),
    installWhisper: (model: string) => call<WhisperStatus>("whisper_install", { model }),
  },

  overlay: {
    takeOsd: () => call<OsdMessage | null>("take_osd"),
    alerts: () => call<Alerts>("alerts_list"),
    fitAlerts: (height: number) => call<void>("alerts_fit", { height }),
    closeAlerts: () => call<void>("alerts_close"),
  },

  system: {
    /** Sistem özellikleri (önbellekten; `refresh` ile yeniden okur). */
    info: (refresh = false) => call<Specs>("system_info", { refresh }),
    /** Ekran kartı sürücüsü; `refresh` ile internetten yeniden sorar. */
    driver: (refresh = false) => call<DriverStatus>("driver_status", { refresh }),
    /** Discord'a yapıştırmak için düz yazı. */
    text: () => call<string>("system_info_text"),
  },

  presence: {
    /** Discord durumu ayarı değişti: hemen uygula. */
    refresh: () => call<void>("presence_refresh"),
  },

  media: {
    /** Şimdi çalan (Spotify, YouTube…); yoksa null. */
    now: () => call<NowPlaying | null>("media_now"),
    /** "toggle" | "next" | "prev" | "shuffle" | "repeat" | "seek:<saniye>" */
    control: (action: string) => call<void>("media_control", { action }),
  },

  net: {
    /** Bağlantı testi; `quick`: oyun içi panel için kısa test (~1 sn). */
    test: (quick = false) => call<NetReport>("net_test", { quick }),
  },

  power: {
    status: () => call<PowerPlan | null>("power_status"),
    schedule: (action: PowerPlan["action"], minutes: number) => call<PowerPlan>("power_schedule", { action, minutes }),
    cancel: () => call<void>("power_cancel"),
    /** Son dakika uyarısını kapat; süre işler, zamanı gelince yine uygulanır. */
    dismiss: () => call<void>("power_dismiss"),
    /** Planı beklemeden uygula. */
    now: () => call<void>("power_now"),
  },

  alarm: {
    /** Uyarı melodisini sustur. */
    mute: () => call<void>("alarm_mute"),
    /** Uyarı penceresini kapat (uyarılar yerinde kalır). */
    hide: () => call<void>("alarm_hide"),
    /** Melodiyi 5 sn dinlet. */
    preview: () => call<void>("alarm_preview"),
  },
};

/** Bu pencerenin görevi: ana pencere, gösterge, uyarılar, hızlı not ya da yapışkan not. */
export function currentView(): { kind: "main" | "osd" | "alerts" | "alarm" | "quicknote" | "sticky" | "dikte" | "panel"; id?: string } {
  let label = "main";
  if (inTauri) {
    const meta = (window as unknown as { __TAURI_INTERNALS__: { metadata?: { currentWindow?: { label?: string } } } })
      .__TAURI_INTERNALS__.metadata;
    label = meta?.currentWindow?.label ?? "main";
  } else {
    label = new URLSearchParams(location.search).get("view") ?? "main";
  }
  if (label.startsWith("sticky-")) return { kind: "sticky", id: label.slice(7) };
  if (label === "osd" || label === "alerts" || label === "alarm" || label === "quicknote" || label === "dikte" || label === "panel") return { kind: label };
  return { kind: "main" };
}

export function isVideo(path: string): boolean {
  return /\.(mp4|webm)$/i.test(path);
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toLocaleString("tr-TR", { maximumFractionDigits: 0 })} KB`;
  return `${(bytes / 1024 / 1024).toLocaleString("tr-TR", { maximumFractionDigits: 1 })} MB`;
}
