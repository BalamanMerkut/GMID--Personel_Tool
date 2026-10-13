<script lang="ts">
  // Çark & Çekiliş: bir liste yaz (isimler, filmler, oyunlar), sonra
  //  • Çark: çevir, ibrenin gösterdiği kazanır (ilk çıkan)
  //  • Eleme: her çevirişte biri elenir, son kalan kazanır
  //  • Kura: N kazanan seç, sıraya diz ya da takımlara böl
  //  • Zar & yazı-tura: zar, yazı-tura, aralıktan sayı
  // Rastgelelik kriptografik ve eşit (fun.ts). Listeler ve son sonuçlar kayıtlı kalır.
  import { onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import Wheel from "../components/Wheel.svelte";
  import { pickMany, randInt, shuffle, sliceColor, splitItems, teams, WHEEL_COLORS } from "../fun";
  import { getJson, setJson } from "../settings.svelte";
  import { notify } from "../toast.svelte";

  type Item = { id: string; name: string };
  type Mode = "wheel" | "elim" | "draw" | "dice";
  type Result = { mode: Mode; title: string; lines: string[]; teams?: string[][]; text: string };
  type Saved = { name: string; items: string[] };
  type Past = { at: number; text: string };

  const MAX = 100;
  const MODES: { id: Mode; label: string; icon: "wheel" | "trophy" | "users" | "dice"; hint: string }[] = [
    { id: "wheel", label: "Çark", icon: "wheel", hint: "Çevir; ibrenin gösterdiği kazanır." },
    { id: "elim", label: "Eleme", icon: "trophy", hint: "Her çevirişte ibrenin gösterdiği elenir; son kalan kazanır." },
    { id: "draw", label: "Kura", icon: "users", hint: "Rastgele kazanan seç, sıraya diz ya da takımlara böl." },
    { id: "dice", label: "Zar & Yazı-tura", icon: "dice", hint: "Listeye gerek yok: zar at, yazı-tura, sayı seç." },
  ];

  const mk = (name: string): Item => ({ id: crypto.randomUUID(), name });

  let mode = $state<Mode>(getJson<Mode>("fun.mode", "wheel"));
  let items = $state<Item[]>(getJson<string[]>("fun.items", []).map(mk));
  let saved = $state<Saved[]>(getJson<Saved[]>("fun.lists", []));
  let history = $state<Past[]>(getJson<Past[]>("fun.history", []));
  let sound = $state(getJson("fun.sound", true));
  let removeWinner = $state(getJson("fun.removeWinner", false));
  let auto = $state(false);
  /** Elemede elenenler (sırayla). */
  let out = $state<string[]>([]);
  let lastOut = $state<string | null>(null);
  let busy = $state(false);
  let result = $state<Result | null>(null);
  let big = $state(false);
  let draft = $state("");
  let bulk = $state(false);
  let bulkText = $state("");
  let saving = $state(false);
  let saveName = $state("");
  let drawKind = $state<"pick" | "order" | "teams">("pick");
  let drawCount = $state(1);
  let teamCount = $state(2);
  let slot = $state("");
  let wheel: Wheel | undefined = $state();
  /** Çarkta çıkan, sonraki çevirişte listeden çıkarılacak. */
  let pendingRemove = $state<string | null>(null);

  const live = $derived(mode === "elim" ? items.filter((i) => !out.includes(i.id)) : items);
  /** Çarktaki dilimler: renk listedeki yerine göre (elemede kalanların rengi değişmesin). */
  const slices = $derived(live.map((it) => ({ ...it, color: sliceColor(items.indexOf(it), items.length) })));
  const outNames = $derived(out.map((id) => items.find((i) => i.id === id)?.name ?? "").filter(Boolean));

  function persist() {
    void setJson("fun.items", items.map((i) => i.name));
  }

  function setMode(m: Mode) {
    if (busy) return;
    mode = m;
    result = null;
    lastOut = null;
    auto = false;
    void setJson("fun.mode", m);
  }

  // ---- liste ----
  function add(names: string[]) {
    const room = MAX - items.length;
    if (room <= 0) return notify(`Listede en fazla ${MAX} öğe olabilir`, "info");
    const clean = names.map((n) => n.trim()).filter(Boolean);
    if (clean.length > room) notify(`İlk ${room} tanesi eklendi (en fazla ${MAX})`, "info");
    items = [...items, ...clean.slice(0, room).map(mk)];
    persist();
  }

  function addDraft() {
    if (!draft.trim()) return;
    add(splitItems(draft));
    draft = "";
  }

  function addBulk() {
    add(splitItems(bulkText));
    bulkText = "";
    bulk = false;
  }

  function remove(id: string) {
    items = items.filter((i) => i.id !== id);
    out = out.filter((x) => x !== id);
    persist();
  }

  function clearAll() {
    items = [];
    out = [];
    result = null;
    lastOut = null;
    persist();
  }

  function shuffleList() {
    items = shuffle(items);
    persist();
  }

  /** Listede olmayanları ekler. */
  function fill(names: string[], empty: string) {
    const have = new Set(items.map((i) => i.name.toLocaleLowerCase("tr")));
    const fresh = [...new Set(names)].filter((n) => !have.has(n.toLocaleLowerCase("tr")));
    if (!names.length) return notify(empty, "info");
    if (!fresh.length) return notify("Hepsi zaten listede", "info");
    add(fresh);
    notify(`${Math.min(fresh.length, MAX)} öğe eklendi`, "plus");
  }

  async function fillWatchlist() {
    const [reviews, titles] = await Promise.all([api.reviews.list("title"), api.titles.list()]).catch(() => [[], []] as const);
    const ids = new Set(reviews.filter((r) => r.status === "watchlist").map((r) => r.subjectId));
    fill(
      titles.filter((t) => ids.has(t.id)).map((t) => t.name),
      "Film & Dizi Arşivi'nde \"İzlenecek\" durumunda bir şey yok",
    );
  }

  async function fillWishlist() {
    const [reviews, games] = await Promise.all([api.reviews.list("game"), api.games.list()]).catch(() => [[], []] as const);
    const ids = new Set(reviews.filter((r) => r.status === "wishlist").map((r) => r.subjectId));
    fill(
      games.filter((g) => ids.has(g.id)).map((g) => g.name),
      "Oyun Arşivi'nde \"İstiyorum\" durumunda oyun yok",
    );
  }

  async function fillInstalled() {
    const games = await api.games.list().catch(() => []);
    fill(
      games.filter((g) => g.installDir).map((g) => g.name),
      "Kütüphanende kurulu oyun bulunamadı",
    );
  }

  function saveList() {
    const name = saveName.trim();
    if (!name || !items.length) return;
    saved = [...saved.filter((s) => s.name !== name), { name, items: items.map((i) => i.name) }];
    void setJson("fun.lists", saved);
    saving = false;
    saveName = "";
    notify(`"${name}" kaydedildi`);
  }

  function loadList(s: Saved) {
    if (busy) return;
    items = s.items.map(mk);
    out = [];
    result = null;
    lastOut = null;
    persist();
  }

  function deleteList(name: string) {
    saved = saved.filter((s) => s.name !== name);
    void setJson("fun.lists", saved);
  }

  // ---- ses ve kutlama ----
  let ctx: AudioContext | null = null;
  function beep(freq: number, dur: number, gain: number, type: OscillatorType = "triangle", delay = 0) {
    if (!sound) return;
    try {
      ctx ??= new AudioContext();
      const t = ctx.currentTime + delay;
      const o = ctx.createOscillator();
      const g = ctx.createGain();
      o.type = type;
      o.frequency.value = freq;
      g.gain.setValueAtTime(gain, t);
      g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
      o.connect(g).connect(ctx.destination);
      o.start(t);
      o.stop(t + dur);
    } catch {
      /* ses aygıtı yoksa sessiz */
    }
  }
  const tick = () => beep(1500, 0.03, 0.035, "square");
  const fanfare = () => [523.25, 659.25, 783.99, 1046.5].forEach((f, i) => beep(f, i === 3 ? 0.7 : 0.3, 0.07, "triangle", i * 0.11));
  const thud = () => beep(180, 0.25, 0.08, "sine");

  type Piece = { x: number; delay: number; dur: number; color: string; size: number; spin: number };
  let confetti = $state<Piece[]>([]);
  let confettiTimer: ReturnType<typeof setTimeout> | undefined;
  function celebrate() {
    fanfare();
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    confetti = Array.from({ length: 80 }, () => ({
      x: randInt(100),
      delay: randInt(400) / 1000,
      dur: 1.6 + randInt(1200) / 1000,
      color: WHEEL_COLORS[randInt(WHEEL_COLORS.length)],
      size: 6 + randInt(6),
      spin: randInt(720) - 360,
    }));
    clearTimeout(confettiTimer);
    confettiTimer = setTimeout(() => (confetti = []), 3400);
  }

  function finish(r: Result, party = true) {
    result = r;
    history = [{ at: Date.now(), text: r.text }, ...history].slice(0, 15);
    void setJson("fun.history", history);
    if (party) celebrate();
  }

  // ---- çark ve eleme ----
  async function spin() {
    if (busy || !wheel) return;
    if (pendingRemove) {
      const id = pendingRemove;
      pendingRemove = null;
      if (removeWinner) remove(id);
    }
    const list = live;
    if (!list.length) return;
    if (mode === "elim" && list.length === 1) return;
    busy = true;
    if (mode === "wheel") result = null;
    const k = randInt(list.length);
    await wheel.spin(k, mode === "elim" && auto ? 2600 : 5200);
    const hit = list[k];
    busy = false;
    if (mode === "wheel") {
      pendingRemove = hit.id;
      finish({ mode, title: "Çarktan çıkan", lines: [hit.name], text: `🎡 **Çark** (${list.length} seçenek): **${hit.name}** çıktı!` });
      return;
    }
    out = [...out, hit.id];
    lastOut = hit.name;
    const left = items.filter((i) => !out.includes(i.id));
    if (left.length === 1) {
      auto = false;
      const order = [...outNames];
      finish({
        mode,
        title: "Son kalan",
        lines: [left[0].name],
        text: `🏆 **Eleme çarkı** (${items.length} seçenek): son kalan **${left[0].name}**!\nElenme sırası: ${order.join(" → ")}`,
      });
    } else {
      thud();
      if (auto) setTimeout(() => auto && void spin(), 650);
    }
  }

  function restartElim() {
    if (busy) return;
    out = [];
    lastOut = null;
    result = null;
    auto = false;
  }

  function removeHit() {
    if (!pendingRemove) return;
    remove(pendingRemove);
    pendingRemove = null;
    result = null;
  }

  // ---- kura ----
  const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

  async function shuffleSlot(names: string[], ms: number) {
    const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    let t = 0;
    let delay = 45;
    while (t < (reduce ? 200 : ms)) {
      slot = names[randInt(names.length)];
      tick();
      await wait(delay);
      t += delay;
      delay = Math.min(220, delay * 1.09);
    }
    slot = "";
  }

  async function runDraw() {
    const names = items.map((i) => i.name);
    if (busy || !names.length) return;
    busy = true;
    result = null;
    await shuffleSlot(names, 1600);
    busy = false;
    if (drawKind === "pick") {
      const n = Math.min(Math.max(1, drawCount), names.length);
      const win = pickMany(names, n);
      finish({
        mode,
        title: n > 1 ? `${n} kazanan` : "Kazanan",
        lines: win,
        text: `🎲 **Kura** (${names.length} kişi/seçenek arasından): ${win.map((w) => `**${w}**`).join(", ")}`,
      });
    } else if (drawKind === "order") {
      const order = shuffle(names);
      finish(
        { mode, title: "Sıra", lines: order, text: `🎲 **Sıra:**\n${order.map((n, i) => `${i + 1}. ${n}`).join("\n")}` },
        false,
      );
      fanfare();
    } else {
      const k = Math.min(Math.max(2, teamCount), names.length);
      const t = teams(names, k);
      finish(
        {
          mode,
          title: `${t.length} takım`,
          lines: [],
          teams: t,
          text: `🎲 **Takımlar**\n${t.map((x, i) => `**Takım ${i + 1}:** ${x.join(", ")}`).join("\n")}`,
        },
        false,
      );
      fanfare();
    }
  }

  // ---- zar & yazı-tura ----
  const FACES = ["⚀", "⚁", "⚂", "⚃", "⚄", "⚅"];
  let coin = $state<"Yazı" | "Tura" | null>(null);
  let flipping = $state(false);
  let flips = $state(0);
  let diceCount = $state(2);
  let dice = $state<number[]>([]);
  let rolling = $state(false);
  let lo = $state(1);
  let hi = $state(100);
  let number = $state<number | null>(null);
  let numberSpin = $state(false);

  function log(text: string) {
    history = [{ at: Date.now(), text }, ...history].slice(0, 15);
    void setJson("fun.history", history);
  }

  async function flip() {
    if (flipping) return;
    flipping = true;
    const face = randInt(2) === 0 ? "Yazı" : "Tura";
    // Para yarım turlarla döner: toplam yarım tur çiftse yazı, tekse tura yüzü üstte kalır.
    let next = flips + 4;
    if ((next % 2 === 1) !== (face === "Tura")) next++;
    flips = next;
    coin = null;
    await wait(matchMedia("(prefers-reduced-motion: reduce)").matches ? 100 : 1250);
    coin = face;
    flipping = false;
    beep(880, 0.18, 0.06);
    log(`🪙 **Yazı-tura:** ${face}`);
  }

  async function roll() {
    if (rolling) return;
    rolling = true;
    const n = Math.min(6, Math.max(1, diceCount));
    for (let i = 0; i < 12; i++) {
      dice = Array.from({ length: n }, () => randInt(6) + 1);
      tick();
      await wait(55 + i * 6);
    }
    dice = Array.from({ length: n }, () => randInt(6) + 1);
    rolling = false;
    beep(660, 0.15, 0.06);
    const sum = dice.reduce((a, b) => a + b, 0);
    log(`🎲 **Zar** (${n}): ${dice.join(" + ")}${n > 1 ? ` = **${sum}**` : ""}`);
  }

  async function pickNumber() {
    if (numberSpin) return;
    const a = Math.min(lo, hi);
    const b = Math.max(lo, hi);
    if (!Number.isFinite(a) || !Number.isFinite(b) || b - a > 1_000_000_000) return notify("Aralık geçersiz", "info");
    numberSpin = true;
    for (let i = 0; i < 10; i++) {
      number = a + randInt(b - a + 1);
      tick();
      await wait(50 + i * 8);
    }
    number = a + randInt(b - a + 1);
    numberSpin = false;
    beep(660, 0.15, 0.06);
    log(`🔢 **Sayı** (${a}–${b}): **${number}**`);
  }

  // ---- ortak ----
  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      notify("Kopyalandı · Discord'da Ctrl+V", "copy");
    } catch (e) {
      notify(String(e), "info");
    }
  }

  function toggleSound() {
    sound = !sound;
    void setJson("fun.sound", sound);
  }

  function onkey(e: KeyboardEvent) {
    const el = e.target as HTMLElement | null;
    if (el && (el.closest("input, textarea, select, button") || el.isContentEditable)) return;
    if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      if (mode === "wheel" || mode === "elim") void spin();
      else if (mode === "draw") void runDraw();
    } else if (e.key === "Escape" && big) big = false;
  }

  const timeOf = (ms: number) => new Date(ms).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" });
  const hint = $derived(MODES.find((m) => m.id === mode)?.hint ?? "");

  onMount(() => () => {
    auto = false;
    clearTimeout(confettiTimer);
    void ctx?.close();
  });
</script>

<svelte:window onkeydown={onkey} />

<div class="fun" class:big>
  {#if !big}
    <aside class="side">
      <section class="panel list">
        <div class="head">
          <h3 class="h2"><Icon name="list" />Liste <span class="count tnum">{items.length}</span></h3>
          <div class="row">
            <button class="btn ghost sm" disabled={busy || items.length < 2} onclick={shuffleList} title="Listeyi karıştır"><Icon name="shuffle" size={14} /></button>
            <button class="btn ghost sm" disabled={busy || !items.length} onclick={clearAll}>Temizle</button>
          </div>
        </div>

        <form class="adder" onsubmit={(e) => (e.preventDefault(), addDraft())}>
          <input bind:value={draft} placeholder="İsim, film, oyun… yaz, Enter" maxlength="80" disabled={busy} />
          <button class="btn sm" disabled={busy || !draft.trim()}><Icon name="plus" size={15} />Ekle</button>
        </form>
        <button class="link" onclick={() => (bulk = !bulk)}>{bulk ? "Toplu eklemeyi kapat" : "Toplu ekle (her satıra bir tane ya da virgülle)"}</button>
        {#if bulk}
          <textarea bind:value={bulkText} rows="5" placeholder={"Interstellar\nDune\nMatrix\n…"}></textarea>
          <button class="btn sm" disabled={!bulkText.trim()} onclick={addBulk}>Hepsini ekle</button>
        {/if}

        <div class="quick">
          <span class="sub small">Hızlı doldur:</span>
          <button class="chip" disabled={busy} onclick={fillWatchlist}><Icon name="film" size={13} />İzlenecek filmler</button>
          <button class="chip" disabled={busy} onclick={fillWishlist}><Icon name="star" size={13} />İstediğim oyunlar</button>
          <button class="chip" disabled={busy} onclick={fillInstalled}><Icon name="library" size={13} />Kurulu oyunlar</button>
        </div>

        {#if items.length}
          <ol class="items">
            {#each items as it, i (it.id)}
              <li class:out={mode === "elim" && out.includes(it.id)}>
                <span class="sw" style:background={sliceColor(i, items.length)}></span>
                <span class="nm">{it.name}</span>
                <button class="x" disabled={busy} onclick={() => remove(it.id)} title="Çıkar" aria-label="{it.name} çıkar"><Icon name="x" size={13} /></button>
              </li>
            {/each}
          </ol>
        {:else}
          <p class="sub small empty">
            Örnek: film gecesi, 6 kişi 6 film söyledi. Hepsini yaz, sonra çarkı çevir ya da elemeyle son kalanı izleyin.
          </p>
        {/if}

        <div class="saved">
          {#each saved as s (s.name)}
            <span class="chip lst">
              <button class="ln" disabled={busy} onclick={() => loadList(s)} title="{s.items.length} öğe: yükle">{s.name}</button>
              <button class="x" onclick={() => deleteList(s.name)} aria-label="{s.name} listesini sil"><Icon name="x" size={11} /></button>
            </span>
          {/each}
          {#if saving}
            <form class="adder" onsubmit={(e) => (e.preventDefault(), saveList())}>
              <!-- svelte-ignore a11y_autofocus -->
              <input bind:value={saveName} placeholder="Liste adı (ör. Film gecesi)" maxlength="40" autofocus />
              <button class="btn sm" disabled={!saveName.trim()}>Kaydet</button>
            </form>
          {:else}
            <button class="chip" disabled={!items.length} onclick={() => (saving = true)}><Icon name="pin" size={13} />Listeyi kaydet</button>
          {/if}
        </div>
      </section>

      <section class="panel hist">
        <div class="head">
          <h3 class="h2"><Icon name="clock" />Son sonuçlar</h3>
          {#if history.length}<button class="link" onclick={() => ((history = []), void setJson("fun.history", []))}>Temizle</button>{/if}
        </div>
        {#if history.length}
          <ul>
            {#each history as h (h.at)}
              <li>
                <span class="tnum t">{timeOf(h.at)}</span>
                <span class="ht">{h.text.replace(/\*\*/g, "").split("\n")[0]}</span>
                <button class="x" onclick={() => copy(h.text)} title="Discord'a kopyala"><Icon name="copy" size={13} /></button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="sub small">Henüz sonuç yok.</p>
        {/if}
      </section>
    </aside>
  {/if}

  <section class="panel stage">
    <div class="top">
      <div class="seg modes">
        {#each MODES as m (m.id)}
          <button aria-pressed={mode === m.id} disabled={busy} onclick={() => setMode(m.id)}>
            <Icon name={m.icon} size={15} />{m.label}
          </button>
        {/each}
      </div>
      <div class="row">
        <button class="btn ghost sm" onclick={toggleSound} title={sound ? "Sesi kapat" : "Sesi aç"}><Icon name={sound ? "speaker" : "mute"} size={15} /></button>
        <button class="btn ghost sm" onclick={() => (big = !big)} title={big ? "Küçült (Esc)" : "Büyük göster: Discord'da ekran paylaşırken"}>
          <Icon name={big ? "shrink" : "expand"} size={15} />{big ? "Küçült" : "Büyük göster"}
        </button>
      </div>
    </div>
    <p class="sub small hint">{hint}</p>

    {#if mode === "wheel" || mode === "elim"}
      <div class="wheelbox">
        <div class="wheelwrap">
          <Wheel bind:this={wheel} items={slices} {busy} ontick={tick} onhub={() => void spin()} />
        </div>
        <div class="info">
          {#if result}
            <div class="result gold">
              <span class="kind"><Icon name="trophy" size={16} />{result.title}</span>
              <b class="win">{result.lines[0]}</b>
              <div class="row">
                <button class="btn sm" onclick={() => copy(result!.text)}><Icon name="discord" size={15} />Discord'a kopyala</button>
                {#if mode === "wheel" && pendingRemove && !removeWinner}
                  <button class="btn ghost sm" onclick={removeHit}>Listeden çıkar</button>
                {/if}
                {#if mode === "elim"}
                  <button class="btn ghost sm" onclick={restartElim}><Icon name="replay" size={14} />Baştan</button>
                {/if}
              </div>
            </div>
          {:else if mode === "elim" && lastOut}
            <div class="result out">
              <span class="kind">Elendi</span>
              <b class="win small">{lastOut}</b>
              <span class="sub small tnum">{live.length} kaldı</span>
            </div>
          {/if}

          {#if !live.length}
            <p class="sub">Önce listeye en az iki şey ekle.</p>
          {:else if mode === "elim" && live.length === 1}
            {#if !result}<p class="sub">Tek seçenek var.</p>{/if}
          {:else}
            <button class="btn go" disabled={busy || (mode === "elim" && live.length < 2)} onclick={() => void spin()}>
              <Icon name="wheel" size={18} />{busy ? "Dönüyor…" : mode === "elim" ? (out.length ? "Sıradakini ele" : "Elemeye başla") : "Çevir"}
            </button>
            <span class="sub small">Boşluk tuşu da çevirir.</span>
          {/if}

          {#if mode === "wheel"}
            <label class="opt"><input type="checkbox" bind:checked={removeWinner} onchange={() => void setJson("fun.removeWinner", removeWinner)} />Çıkan, sonraki çevirişte listeden çıksın</label>
          {:else}
            <label class="opt"><input type="checkbox" bind:checked={auto} onchange={() => auto && !busy && live.length > 1 && !result && void spin()} />Otomatik: son kalana kadar kendisi çevirsin</label>
            {#if out.length && !result}
              <button class="link" disabled={busy} onclick={restartElim}>Baştan başla</button>
            {/if}
            {#if outNames.length}
              <ol class="outs">
                {#each outNames as n, i (i)}<li><span class="tnum">{i + 1}.</span> {n}</li>{/each}
              </ol>
            {/if}
          {/if}
        </div>
      </div>
    {:else if mode === "draw"}
      <div class="draw">
        <div class="seg">
          <button aria-pressed={drawKind === "pick"} disabled={busy} onclick={() => ((drawKind = "pick"), (result = null))}>Kazanan seç</button>
          <button aria-pressed={drawKind === "order"} disabled={busy} onclick={() => ((drawKind = "order"), (result = null))}>Sıraya diz</button>
          <button aria-pressed={drawKind === "teams"} disabled={busy} onclick={() => ((drawKind = "teams"), (result = null))}>Takımlara böl</button>
        </div>
        <div class="row">
          {#if drawKind === "pick"}
            <label class="num">Kaç kazanan <input type="number" min="1" max={Math.max(1, items.length)} bind:value={drawCount} /></label>
          {:else if drawKind === "teams"}
            <label class="num">Kaç takım <input type="number" min="2" max={Math.max(2, items.length)} bind:value={teamCount} /></label>
          {/if}
          <button class="btn go" disabled={busy || items.length < 2} onclick={() => void runDraw()}>
            <Icon name="dice" size={18} />{busy ? "Çekiliyor…" : drawKind === "pick" ? "Kura çek" : drawKind === "order" ? "Sırala" : "Takımları kur"}
          </button>
          {#if items.length < 2}<span class="sub small">Önce listeye en az iki şey ekle.</span>{/if}
        </div>

        {#if slot}
          <div class="slot"><b>{slot}</b></div>
        {:else if result}
          <div class="result gold">
            <span class="kind"><Icon name="trophy" size={16} />{result.title}</span>
            {#if result.teams}
              <div class="teams">
                {#each result.teams as t, i (i)}
                  <div class="team" style:--c={WHEEL_COLORS[i % WHEEL_COLORS.length]}>
                    <b>Takım {i + 1}</b>
                    <ul>{#each t as n, j (j)}<li>{n}</li>{/each}</ul>
                  </div>
                {/each}
              </div>
            {:else if drawKind === "order"}
              <ol class="order">{#each result.lines as n, i (i)}<li style:animation-delay="{i * 70}ms">{n}</li>{/each}</ol>
            {:else}
              <div class="wins">{#each result.lines as n, i (i)}<b class="win" style:animation-delay="{i * 120}ms">{n}</b>{/each}</div>
            {/if}
            <div class="row"><button class="btn sm" onclick={() => copy(result!.text)}><Icon name="discord" size={15} />Discord'a kopyala</button></div>
          </div>
        {/if}
      </div>
    {:else}
      <div class="tools">
        <div class="tool">
          <h4><Icon name="coin" size={16} />Yazı-tura</h4>
          <div class="coin" style:transform="rotateY({flips * 180}deg)" class:flipping>
            <span class="face front">Y</span><span class="face back">T</span>
          </div>
          <b class="big-r">{coin ?? (flipping ? "…" : "")}</b>
          <button class="btn" disabled={flipping} onclick={() => void flip()}>At</button>
        </div>
        <div class="tool">
          <h4><Icon name="dice" size={16} />Zar</h4>
          <div class="dice" class:rolling>
            {#each dice.length ? dice : Array(Math.min(6, Math.max(1, diceCount))).fill(0) as d, i (i)}
              <span class="die">{d ? FACES[d - 1] : "·"}</span>
            {/each}
          </div>
          <b class="big-r tnum">{dice.length > 1 && !rolling ? `Toplam ${dice.reduce((a, b) => a + b, 0)}` : ""}</b>
          <div class="row">
            <label class="num">Zar sayısı <input type="number" min="1" max="6" bind:value={diceCount} /></label>
            <button class="btn" disabled={rolling} onclick={() => void roll()}>At</button>
          </div>
        </div>
        <div class="tool">
          <h4><Icon name="sparkle" size={16} />Sayı seç</h4>
          <b class="numout tnum">{number ?? "?"}</b>
          <div class="row">
            <label class="num"><input type="number" bind:value={lo} aria-label="En az" /></label>
            <span class="sub">–</span>
            <label class="num"><input type="number" bind:value={hi} aria-label="En çok" /></label>
            <button class="btn" disabled={numberSpin} onclick={() => void pickNumber()}>Seç</button>
          </div>
        </div>
      </div>
    {/if}
  </section>
</div>

{#if confetti.length}
  <div class="confetti" aria-hidden="true">
    {#each confetti as p, i (i)}
      <i style:left="{p.x}%" style:background={p.color} style:width="{p.size}px" style:height="{p.size * 0.45}px" style:animation-delay="{p.delay}s" style:animation-duration="{p.dur}s" style:--spin="{p.spin}deg"></i>
    {/each}
  </div>
{/if}

<style>
  .fun { display: grid; grid-template-columns: minmax(280px, 360px) minmax(0, 1fr); gap: 16px; max-width: 1500px; margin: 0 auto; align-items: start; }
  .fun.big { grid-template-columns: minmax(0, 1fr); }
  .side { display: flex; flex-direction: column; gap: 16px; min-width: 0; }
  .panel { display: flex; flex-direction: column; gap: 12px; }
  .head { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
  .count { font-size: 12px; color: var(--dim); font-weight: 600; }
  .sm { padding: 5px 10px; font-size: 12.5px; }
  .adder { display: flex; gap: 8px; }
  input, textarea { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 8px 10px; color: var(--text); min-width: 0; }
  .adder input { flex: 1; }
  textarea { resize: vertical; font: inherit; }
  .link { background: none; border: 0; padding: 0; color: var(--gold-dim); font-size: 12.5px; text-align: left; align-self: flex-start; }
  .link:hover { color: var(--gold); }
  .quick, .saved { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .chip { cursor: pointer; }
  .chip:hover:not(:disabled) { color: var(--gold-glow); border-color: var(--gold-dim); }
  .chip:disabled { opacity: 0.5; cursor: default; }
  .lst { padding: 0 4px 0 0; gap: 0; }
  .lst .ln { background: none; border: 0; color: inherit; padding: 4px 6px 4px 10px; font-size: 12px; }
  .saved .adder { flex: 1 1 100%; }
  .items { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 3px; max-height: 360px; overflow: auto; }
  .items li { display: flex; align-items: center; gap: 8px; padding: 5px 6px 5px 8px; border-radius: 8px; background: rgba(255, 255, 255, 0.025); font-size: 13.5px; }
  .items li.out { opacity: 0.45; }
  .items li.out .nm { text-decoration: line-through; }
  .sw { width: 8px; height: 8px; border-radius: 2px; flex: none; }
  .nm { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .x { background: none; border: 0; color: var(--dim); padding: 3px; border-radius: 6px; display: grid; place-items: center; }
  .x:hover:not(:disabled) { color: var(--bad); background: rgba(224, 106, 106, 0.1); }
  .empty { line-height: 1.5; }
  .hist ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 2px; max-height: 260px; overflow: auto; }
  .hist li { display: flex; align-items: center; gap: 8px; font-size: 12.5px; padding: 4px 0; border-top: 1px solid rgba(68, 74, 81, 0.4); }
  .hist li:first-child { border-top: 0; }
  .hist .t { color: var(--dim); flex: none; }
  .ht { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .stage { gap: 10px; min-height: 520px; }
  .top { display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; }
  .modes button { display: inline-flex; align-items: center; gap: 6px; }
  .hint { margin-top: -2px; }
  .wheelbox { display: grid; grid-template-columns: minmax(0, 1.4fr) minmax(220px, 1fr); gap: 24px; align-items: center; padding: 8px 0; }
  .big .wheelbox { grid-template-columns: minmax(0, 2fr) minmax(240px, 1fr); }
  .wheelwrap { padding-top: 10px; }
  .big .wheelwrap :global(.wheel) { max-width: min(78vh, 820px); }
  .info { display: flex; flex-direction: column; gap: 12px; align-items: flex-start; }
  .go { padding: 12px 22px; font-size: 16px; }
  .opt { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--muted); }
  .opt input { accent-color: var(--gold); width: 16px; height: 16px; }
  .result {
    display: flex; flex-direction: column; gap: 8px; padding: 16px 18px; border-radius: var(--r-lg); width: 100%;
    border: 1px solid var(--line-2); background: rgba(255, 255, 255, 0.03); animation: pop 0.35s ease-out;
  }
  .result.gold { border-color: rgba(227, 194, 112, 0.7); background: linear-gradient(160deg, rgba(227, 194, 112, 0.16), rgba(34, 37, 42, 0.6)); box-shadow: 0 0 30px rgba(227, 194, 112, 0.12); }
  .result.out { border-color: rgba(224, 106, 106, 0.4); }
  .kind { display: flex; align-items: center; gap: 6px; font-family: var(--f-display); font-size: 11px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; color: var(--gold); }
  .out .kind { color: var(--bad); }
  .win { font-family: var(--f-display); font-size: clamp(22px, 3vw, 34px); line-height: 1.15; overflow-wrap: anywhere; animation: pop 0.4s ease-out both; }
  .win.small { font-size: 20px; }
  .outs { margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column; gap: 3px; font-size: 13px; color: var(--muted); max-height: 220px; overflow: auto; }
  .outs span { color: var(--dim); }

  .draw { display: flex; flex-direction: column; gap: 14px; padding: 6px 0; }
  .num { display: flex; align-items: center; gap: 6px; color: var(--muted); font-size: 13px; }
  .num input { width: 76px; }
  .slot { display: grid; place-items: center; min-height: 180px; border-radius: var(--r-lg); border: 1px dashed var(--line-2); }
  .slot b { font-family: var(--f-display); font-size: clamp(26px, 4vw, 44px); color: var(--gold-glow); text-align: center; padding: 0 12px; }
  .wins { display: flex; flex-wrap: wrap; gap: 8px 22px; }
  .order { margin: 0; padding-left: 22px; display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 6px 18px; font-size: 15px; }
  .order li { animation: pop 0.3s ease-out both; }
  .teams { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 10px; }
  .team { border-radius: 12px; padding: 10px 12px; border: 1px solid var(--line); border-top: 3px solid var(--c); background: rgba(255, 255, 255, 0.03); animation: pop 0.3s ease-out both; }
  .team ul { margin: 6px 0 0; padding-left: 18px; font-size: 14px; display: flex; flex-direction: column; gap: 3px; }

  .tools { display: grid; grid-template-columns: repeat(auto-fit, minmax(230px, 1fr)); gap: 14px; padding: 6px 0; }
  .tool { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 18px; border-radius: var(--r-lg); border: 1px solid var(--line); background: rgba(255, 255, 255, 0.02); }
  .tool h4 { margin: 0; display: flex; align-items: center; gap: 6px; color: var(--gold); font-size: 14px; align-self: flex-start; }
  .big-r { min-height: 26px; font-family: var(--f-display); font-size: 20px; }
  .coin { position: relative; width: 96px; height: 96px; transform-style: preserve-3d; transition: transform 1.2s cubic-bezier(0.2, 0.7, 0.2, 1); }
  .face {
    position: absolute; inset: 0; border-radius: 50%; display: grid; place-items: center; backface-visibility: hidden;
    font-family: var(--f-display); font-weight: 800; font-size: 34px; color: #2a2008;
    background: radial-gradient(circle at 35% 30%, var(--gold-glow), var(--gold) 55%, #a8893f); border: 4px solid #c9a95a;
    box-shadow: inset 0 0 0 4px rgba(255, 255, 255, 0.15);
  }
  .face.back { transform: rotateY(180deg); }
  .dice { display: flex; flex-wrap: wrap; justify-content: center; gap: 6px; min-height: 64px; }
  .die { font-size: 58px; line-height: 1; color: var(--text); }
  .rolling .die { animation: shake 0.12s linear infinite; color: var(--gold-glow); }
  .numout { font-family: var(--f-display); font-size: 48px; color: var(--gold-glow); min-height: 58px; }

  .confetti { position: fixed; inset: 0; pointer-events: none; z-index: 60; overflow: hidden; }
  .confetti i { position: absolute; top: -12px; border-radius: 2px; animation: fall linear forwards; }
  @keyframes fall { to { transform: translateY(105vh) rotate(var(--spin)); opacity: 0.9; } }
  @keyframes pop { from { opacity: 0; transform: scale(0.94); } }
  @keyframes shake { 50% { transform: rotate(8deg) translateY(-2px); } }
  @media (max-width: 1100px) { .wheelbox { grid-template-columns: minmax(0, 1fr); } .info { align-items: stretch; } }
  @media (max-width: 900px) { .fun { grid-template-columns: minmax(0, 1fr); } }
</style>
