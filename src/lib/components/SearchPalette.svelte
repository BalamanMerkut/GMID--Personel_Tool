<script lang="ts">
  // Her şeyde tek arama (Ctrl+K): ekranlar ve ayarlar burada, notlar/oyunlar/filmler/videolar/klipler çekirdekte aranır.
  // Ok tuşlarıyla gezilir, Enter açar, Esc kapatır.
  import { tick } from "svelte";
  import { api } from "../api";
  import { requestFocus } from "../focus.svelte";
  import Icon from "../Icon.svelte";
  import type { IconName } from "../icons";
  import { SCREENS } from "../nav";
  import { play } from "../player.svelte";
  import { notify } from "../toast.svelte";
  import type { SearchHit } from "../types";

  let { open = $bindable(false), onnavigate }: { open: boolean; onnavigate: (route: string) => void } = $props();

  type Row = { key: string; group: string; icon: IconName; title: string; sub: string; image: string | null; go: () => void };

  const SETTINGS: [string, string, string][] = [
    ["genel", "Genel", "başlangıç windows açılış tepsi pencere kapat"],
    ["genel", "Bilgisayarı zamanla kapat", "kapat zamanla yeniden başlat uyku uyut kapanış shutdown"],
    ["sistem", "Sistem özellikleri", "sistem donanım işlemci ekran kartı ram anakart disk monitör fare klavye"],
    ["gorunum", "Görünüm", "arka plan resim gif video renk tema"],
    ["kisayollar", "Kısayollar", "kısayol tuş klavye hotkey"],
    ["dikte", "Sesle yazma", "dikte konuş whisper mikrofon"],
    ["ses", "Ses & Mikrofon", "mikrofon kilit kulaklık hoparlör"],
    ["bildirimler", "Bildirimler", "bildirim uyarı ses"],
    ["indirici", "Video İndirici ayarları", "yt-dlp ffmpeg eklenti chrome"],
    ["oyunlar", "Oyun ayarları", "algılama klip klasör steam"],
    ["veri", "Veri ve yedekler", "yedek geri yükle klasör günlük"],
    ["hakkinda", "Hakkında ve güncelleme", "sürüm güncelle"],
    ["hakkinda", "Sorun bildir", "sorun hata rapor bozuk çalışmıyor destek"],
    ["hakkinda", "Tanıtım turu", "rehber tanıtım nasıl kullanılır yardım başlangıç"],
  ];
  const KIND: Record<SearchHit["kind"], [string, IconName]> = {
    note: ["Notlar", "note"],
    game: ["Oyunlar", "library"],
    title: ["Film & Dizi", "film"],
    video: ["Videolar ve müzikler", "headphones"],
    clip: ["Klipler", "clip"],
    reminder: ["Hatırlatıcılar", "bell"],
  };

  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let active = $state(0);
  let input = $state<HTMLInputElement | null>(null);
  let list = $state<HTMLElement | null>(null);

  const fold = (s: string) =>
    s.toLocaleLowerCase("tr").replace(/ı/g, "i").replace(/ş/g, "s").replace(/ğ/g, "g").replace(/ç/g, "c").replace(/ö/g, "o").replace(/ü/g, "u");
  const matches = (hay: string, q: string) => fold(q).split(/\s+/).filter(Boolean).every((w) => fold(hay).includes(w));

  $effect(() => {
    if (open) {
      query = "";
      hits = [];
      active = 0;
      void tick().then(() => input?.focus());
    }
  });

  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const q = query.trim();
    clearTimeout(timer);
    if (q.length < 2) {
      hits = [];
      return;
    }
    timer = setTimeout(() => {
      void api.search(q).then((h) => {
        if (query.trim() === q) hits = h;
      }).catch(() => {});
    }, 120);
  });

  function close() {
    open = false;
  }

  async function openHit(h: SearchHit) {
    close();
    switch (h.kind) {
      case "note":
        requestFocus("note", h.id);
        onnavigate("notes");
        break;
      case "game":
        requestFocus("game", h.id);
        onnavigate(h.sub.startsWith("Oyun Arşivi") ? "archive" : "library");
        break;
      case "title":
        requestFocus("title", h.id);
        onnavigate("films");
        break;
      case "clip":
        requestFocus("clip", h.id);
        onnavigate("clips");
        break;
      case "reminder":
        onnavigate("reminders");
        break;
      case "video": {
        const all = await api.videos.list().catch(() => []);
        const item = all.find((d) => d.id === h.id);
        if (item) play(item, all.filter((d) => d.status === "done" && d.audioOnly === item.audioOnly));
        else notify("Video bulunamadı", "info");
        break;
      }
    }
  }

  const rows = $derived.by((): Row[] => {
    const q = query.trim();
    const out: Row[] = [];
    if (!q) {
      // Boşken: ekranlar (hızlı geçiş).
      for (const s of SCREENS.filter((s) => s.id !== "home").slice(0, 8)) {
        out.push({ key: `s-${s.id}`, group: "Ekranlar", icon: s.icon, title: s.label, sub: "", image: null, go: () => (close(), onnavigate(s.id)) });
      }
      return out;
    }
    for (const s of SCREENS.filter((s) => matches(`${s.label} ${s.description}`, q)).slice(0, 5)) {
      out.push({ key: `s-${s.id}`, group: "Ekranlar", icon: s.icon, title: s.label, sub: "", image: null, go: () => (close(), onnavigate(s.id)) });
    }
    for (const [id, label, keys] of SETTINGS.filter(([, l, k]) => matches(`${l} ${k}`, q)).slice(0, 3)) {
      out.push({ key: `t-${id}-${label}`, group: "Kontrol Odası", icon: "gear", title: label, sub: "", image: null, go: () => (close(), onnavigate(`settings/${id}`)) });
    }
    for (const h of hits) {
      const [group, icon] = KIND[h.kind];
      out.push({ key: `${h.kind}-${h.id}`, group, icon, title: h.title, sub: h.sub, image: h.image, go: () => void openHit(h) });
    }
    return out;
  });

  $effect(() => {
    void rows.length;
    active = Math.min(active, Math.max(0, rows.length - 1));
  });

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      active = Math.min(rows.length - 1, active + 1);
      scroll();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      active = Math.max(0, active - 1);
      scroll();
    } else if (e.key === "Enter") {
      e.preventDefault();
      rows[active]?.go();
    }
  }

  function scroll() {
    void tick().then(() => list?.querySelector(".row.on")?.scrollIntoView({ block: "nearest" }));
  }

  const img = (p: string | null) => (!p ? null : /^(https?:|data:)/.test(p) ? p : api.fileSrc(p));
</script>

{#if open}
  <div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && close()}>
    <div class="pal" role="dialog" aria-label="Ara">
      <label class="field">
        <Icon name="search" size={18} />
        <input bind:this={input} bind:value={query} onkeydown={onkey} placeholder="Ne arıyorsun? Not, oyun, film, video, klip, ekran…" aria-label="Ara" />
        <kbd>Esc</kbd>
      </label>
      <div class="list" bind:this={list}>
        {#each rows as r, i (r.key)}
          {#if i === 0 || rows[i - 1].group !== r.group}<div class="group">{r.group}</div>{/if}
          <button class="row" class:on={i === active} onmouseenter={() => (active = i)} onclick={r.go}>
            {#if r.image}<img src={img(r.image)} alt="" loading="lazy" />{:else}<span class="ic"><Icon name={r.icon} size={16} /></span>{/if}
            <span class="tx"><b>{r.title}</b>{#if r.sub}<span>{r.sub}</span>{/if}</span>
            {#if i === active}<span class="enter">↵</span>{/if}
          </button>
        {:else}
          {#if query.trim().length >= 2}<p class="empty">“{query.trim()}” için bir şey bulunamadı.</p>{/if}
        {/each}
      </div>
      <div class="foot"><span>↑↓ gez</span><span>↵ aç</span><span>Ctrl+K her yerden</span></div>
    </div>
  </div>
{/if}

<style>
  .back { position: fixed; inset: 0; z-index: 90; background: rgba(0, 0, 0, 0.5); backdrop-filter: blur(3px); display: flex; justify-content: center; align-items: flex-start; padding: 12vh 16px 16px; }
  .pal { width: min(640px, 100%); max-height: 70vh; display: flex; flex-direction: column; border-radius: 16px; overflow: hidden; background: var(--night-2); border: 1px solid var(--line-2); box-shadow: 0 30px 80px rgba(0, 0, 0, 0.6); }
  .field { display: flex; align-items: center; gap: 10px; padding: 14px 16px; border-bottom: 1px solid var(--line); color: var(--muted); }
  .field input { flex: 1; min-width: 0; background: none; border: 0; outline: none; color: var(--text); font-size: 16px; }
  kbd { font: 600 11px var(--f-body); padding: 2px 6px; border-radius: 5px; border: 1px solid var(--line-2); color: var(--dim); }
  .list { overflow-y: auto; padding: 6px; }
  .group { padding: 10px 10px 4px; font-size: 11px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--dim); }
  .row { display: flex; align-items: center; gap: 12px; width: 100%; padding: 8px 10px; border: 0; border-radius: 10px; background: none; color: var(--text); text-align: left; }
  .row.on { background: rgba(227, 194, 112, 0.12); }
  .row img, .ic { width: 34px; height: 34px; border-radius: 8px; object-fit: cover; flex: none; background: var(--coal-2); }
  .ic { display: grid; place-items: center; color: var(--gold); }
  .tx { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .tx b { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tx span { font-size: 12px; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .enter { color: var(--gold); font-size: 14px; }
  .empty { margin: 0; padding: 18px; color: var(--muted); text-align: center; }
  .foot { display: flex; gap: 14px; padding: 8px 16px; border-top: 1px solid var(--line); font-size: 11.5px; color: var(--dim); }
</style>
