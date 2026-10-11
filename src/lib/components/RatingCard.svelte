<script lang="ts">
  // Puanlama kartı. Önce durum sorulur, gerisi ona göre gelir:
  //  • İstiyorum: puan yok; neden istediğin ve notlar
  //  • Oynuyorum / Bitirdim / %100 / Bıraktım: 5 kısa soru (7'den başlar), otomatik genel puan (±1), tek cümle,
  //    tavsiye, notlar. Aynı oyunu yeniden puanlamak öncekini günceller.
  import { untrack } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import { GUIDE, QUESTIONS, RECOMMEND, STATUS, autoScore, fmtScore, overallOf, scoreTone } from "../review";
  import { notify } from "../toast.svelte";
  import type { Game, Recommend, Review, ReviewStatus, Scores } from "../types";

  interface Props {
    game: Game;
    review: Review | null;
    onclose: () => void;
    onsaved?: (r: Review) => void;
    ondeleted?: () => void;
  }
  let { game, review, onclose, onsaved, ondeleted }: Props = $props();

  const start = untrack(() => review);
  let scores: Scores = $state(
    start ? { ...start.scores } : { gameplay: 7, story: 7, enjoyment: 7, visuals: 7, performance: 7 },
  );
  // "Yok"tan geri dönünce kaydırıcı son değerine dönsün.
  const lastValue: Record<string, number> = {};
  let adjust = $state(start?.adjust ?? 0);
  let oneLiner = $state(start?.oneLiner ?? "");
  let notes = $state(start?.notes ?? "");
  let status: ReviewStatus | null = $state(start?.status ?? null);
  let recommend: Recommend | null = $state(start?.recommend ?? null);
  let saving = $state(false);
  let armedDelete = $state(false);

  const STATUS_HINT: Record<ReviewStatus, string> = {
    playing: "Hâlâ oynuyorum",
    finished: "Hikâyesini bitirdim",
    completed: "Her şeyini yaptım",
    dropped: "Yarıda bıraktım",
    wishlist: "Henüz oynamadım, oynamak istiyorum",
  };
  // Eski bir puanda durum seçilmemişse her şey görünsün.
  const chosen = $derived(status != null || !!start);
  const wishlist = $derived(status === "wishlist");
  const auto = $derived(autoScore(scores));
  const overall = $derived(overallOf(scores, adjust));

  function toggleNone(key: keyof Scores) {
    if (scores[key] == null) scores[key] = lastValue[key] ?? 7;
    else {
      lastValue[key] = scores[key]!;
      scores[key] = null;
    }
  }

  function nudge(d: number) {
    adjust = Math.max(-1, Math.min(1, Math.round((adjust + d) * 2) / 2));
  }

  async function save() {
    saving = true;
    try {
      const r = await api.reviews.save({
        subjectKind: "game",
        subjectId: game.id,
        scores: $state.snapshot(scores),
        adjust,
        oneLiner: oneLiner.trim() || null,
        notes: notes.trim() || null,
        status,
        recommend,
      });
      notify(r.overall == null ? `${game.name} istek listene eklendi` : `${game.name}: ${fmtScore(r.overall)} / 10 kaydedildi`, "check");
      onsaved?.(r);
      onclose();
    } catch (e) {
      notify(String(e), "info");
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!start) return;
    if (!armedDelete) {
      armedDelete = true;
      return;
    }
    try {
      await api.reviews.remove(start.id);
      notify("Puan silindi", "check");
      ondeleted?.();
      onclose();
    } catch (e) {
      notify(String(e), "info");
    }
  }

  const cover = $derived(!game.cover ? null : /^(https?:|data:)/.test(game.cover) ? game.cover : api.fileSrc(game.cover));
  const genres = $derived([...game.genres, ...game.tags.filter((t) => !game.genres.includes(t))].slice(0, 5));
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="card panel" role="dialog" aria-label={`${game.name} puanla`}>
    <div class="head">
      {#if cover}<img class="cv" src={cover} alt="" />{/if}
      <div class="htx">
        <div class="spread">
          <h3>{game.name}</h3>
          <button class="x" onclick={onclose} title="Kapat"><Icon name="x" size={18} /></button>
        </div>
        {#if genres.length}<span class="sub small">{genres.join(" · ")}</span>{/if}
        <span class="sub small">{[game.year, game.developer].filter(Boolean).join(" · ")}</span>
      </div>
    </div>

    <div class="field status">
      <span>Bu oyunla durumun ne?</span>
      <div class="sts">
        {#each STATUS as s (s.id)}
          <button class="st" aria-pressed={status === s.id} onclick={() => (status = s.id)}>
            <b>{s.label}</b><small>{STATUS_HINT[s.id]}</small>
          </button>
        {/each}
      </div>
    </div>

    {#if !chosen}
      <p class="sub small pick">Önce durumu seç; sorular ona göre gelir.</p>
    {:else}
    {#if !wishlist}
    {#if status === "playing"}<span class="sub small">Şimdiye kadarki izlenimin; bitirince güncelleyebilirsin.</span>{/if}
    <div class="qs">
      {#each QUESTIONS as q (q.key)}
        {@const v = scores[q.key]}
        <div class="q" class:off={v == null}>
          <span class="qi" aria-hidden="true">{q.icon}</span>
          <div class="ql">
            <b>{q.label}</b>
            <span class="sub small">{q.hint} · %{q.weight}</span>
          </div>
          <input
            type="range"
            min="1"
            max="10"
            step="1"
            value={v ?? 7}
            disabled={v == null}
            style={`--p:${(((v ?? 7) - 1) / 9) * 100}%`}
            oninput={(e) => (scores[q.key] = Number(e.currentTarget.value))}
            aria-label={q.label}
          />
          <span class="qv tnum">{v ?? "—"}</span>
          {#if q.none}
            <button class="none" aria-pressed={v == null} onclick={() => toggleNone(q.key)} title="Bu oyunda yok: etkisi diğer sorulara dağılır">Yok</button>
          {:else}
            <span></span>
          {/if}
        </div>
      {/each}
      <span class="sub small guide">{GUIDE}</span>
    </div>

    <div class="total">
      <span class="star" aria-hidden="true">⭐</span>
      <div class="ttx">
        <span class="sub small">Genel puan</span>
        <b class={`tone-${scoreTone(overall ?? 0)}`}>{overall == null ? "—" : fmtScore(overall)}<small> / 10</small></b>
      </div>
      <span class="sub small">
        {#if adjust === 0}otomatik{:else}otomatik {auto == null ? "—" : fmtScore(auto)} · elle {adjust > 0 ? "+" : "−"}{fmtScore(Math.abs(adjust))}{/if}
      </span>
      <div class="nudge">
        <button onclick={() => nudge(-0.5)} disabled={adjust <= -1} title="Yarım puan düşür">−</button>
        <button onclick={() => nudge(0.5)} disabled={adjust >= 1} title="Yarım puan artır">+</button>
      </div>
    </div>

    {/if}

    <label class="field">
      <span>{wishlist ? "Neden oynamak istiyorsun?" : status === "dropped" ? "Neden bıraktın?" : "Tek cümle"}</span>
      <input
        bind:value={oneLiner}
        maxlength="200"
        placeholder={wishlist ? "Arkadaşlar çok övüyor, co-op oynarız." : status === "dropped" ? "Ortası çok uzadı, sıkıldım." : "Bir tur daha derken sabah oldu."}
      />
    </label>

    {#if !wishlist}
    <div class="field">
      <span>Tavsiye</span>
      <div class="chips">
        {#each RECOMMEND as r (r.id)}
          <button class="ch" aria-pressed={recommend === r.id} onclick={() => (recommend = recommend === r.id ? null : r.id)}>{r.icon} {r.label}</button>
        {/each}
      </div>
    </div>

    {/if}

    <label class="field">
      <span>Notlar</span>
      <textarea bind:value={notes} rows="4" placeholder={wishlist ? "Fiyatı, kiminle oynanır, nereden duydun…" : "İpuçları, build'ler, kiminle oynadın… Spoiler'ı ||böyle|| yazarsan gizlenir."}></textarea>
    </label>
    {/if}

    <div class="foot">
      {#if start}
        <button class="link danger" onclick={remove}>{armedDelete ? "Emin misin? Puan silinir" : "Puanı sil"}</button>
      {:else}
        <span></span>
      {/if}
      <div class="row">
        <button class="btn ghost" onclick={onclose}>Vazgeç</button>
        <button class="btn" onclick={save} disabled={saving || !chosen || (!wishlist && overall == null)}>
          <Icon name="check" size={17} />{wishlist ? "İstek listeme ekle" : "Kaydet"}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .back { position: fixed; inset: 0; z-index: 70; background: rgba(0, 0, 0, 0.6); display: grid; place-items: center; padding: 24px; }
  .card { width: min(720px, 100%); max-height: 100%; overflow: auto; display: flex; flex-direction: column; gap: 16px; background: var(--night-2); box-shadow: 0 30px 80px rgba(0, 0, 0, 0.6); }
  .head { display: flex; gap: 16px; align-items: flex-start; }
  .cv { width: 72px; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 10px; flex: none; }
  .htx { flex: 1; display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .htx h3 { margin: 0; font-family: var(--f-display); font-size: 22px; }
  .x { border: 0; background: none; color: var(--muted); padding: 6px; border-radius: 8px; display: grid; }
  .x:hover { color: var(--text); background: rgba(255, 255, 255, 0.07); }

  .qs { display: flex; flex-direction: column; gap: 4px; padding: 10px 12px; border-radius: 14px; background: rgba(255, 255, 255, 0.025); border: 1px solid var(--line); }
  .q { display: grid; grid-template-columns: 26px minmax(0, 1fr) minmax(140px, 220px) 28px 46px; gap: 10px; align-items: center; padding: 6px 0; }
  .qi { font-size: 18px; text-align: center; }
  .ql { display: flex; flex-direction: column; min-width: 0; }
  .ql b { font-weight: 600; font-size: 14.5px; }
  .q.off .ql b, .q.off .qv { color: var(--dim); }
  .qv { font-family: var(--f-display); font-weight: 700; font-size: 18px; text-align: right; color: var(--gold-glow); }
  input[type="range"] { appearance: none; -webkit-appearance: none; height: 6px; border-radius: 3px; background: linear-gradient(90deg, var(--gold) var(--p), rgba(255, 255, 255, 0.1) var(--p)); outline: none; width: 100%; }
  input[type="range"]::-webkit-slider-thumb { -webkit-appearance: none; width: 18px; height: 18px; border-radius: 50%; background: var(--gold-glow); border: 3px solid var(--night-2); box-shadow: 0 0 0 1px var(--gold); cursor: pointer; }
  input[type="range"]:disabled { opacity: 0.3; }
  input[type="range"]:focus-visible { box-shadow: 0 0 0 2px var(--teal); }
  .none { border: 1px solid var(--line-2); background: transparent; color: var(--muted); border-radius: 8px; padding: 3px 0; font-size: 12px; font-weight: 600; }
  .none[aria-pressed="true"] { background: rgba(227, 194, 112, 0.14); color: var(--gold-glow); border-color: var(--gold-dim); }
  .guide { text-align: right; padding-top: 2px; }

  .total { display: flex; align-items: center; gap: 12px; padding: 12px 16px; border-radius: 14px; background: linear-gradient(90deg, rgba(227, 194, 112, 0.12), rgba(227, 194, 112, 0.02)); border: 1px solid var(--gold-dim); flex-wrap: wrap; }
  .star { font-size: 24px; }
  .ttx { display: flex; flex-direction: column; }
  .ttx b { font-family: var(--f-display); font-size: 30px; line-height: 1.1; }
  .ttx small { font-size: 14px; color: var(--muted); font-weight: 500; }
  .total > .sub { flex: 1; }
  .nudge { display: flex; gap: 6px; }
  .nudge button { width: 34px; height: 34px; border-radius: 10px; border: 1px solid var(--line-2); background: var(--coal-2); color: var(--text); font-size: 18px; font-weight: 700; }
  .nudge button:hover:not(:disabled) { border-color: var(--gold); color: var(--gold-glow); }
  .nudge button:disabled { opacity: 0.35; }
  .tone-gold { color: var(--gold-glow); }
  .tone-good { color: var(--good); }
  .tone-mid { color: #e0a84a; }
  .tone-low { color: var(--bad); }

  .field { display: flex; flex-direction: column; gap: 6px; }
  .sts { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 8px; }
  .st { display: flex; flex-direction: column; gap: 2px; text-align: left; border: 1px solid var(--line-2); background: rgba(255, 255, 255, 0.025); color: var(--text); border-radius: 12px; padding: 9px 11px; }
  .st small { color: var(--dim); font-size: 11.5px; line-height: 1.25; }
  .st:hover { border-color: var(--gold-dim); }
  .st[aria-pressed="true"] { background: rgba(227, 194, 112, 0.14); border-color: var(--gold); }
  .st[aria-pressed="true"] b { color: var(--gold-glow); }
  .pick { margin: 0; text-align: center; padding: 18px 0 6px; }
  .field > span { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .field input, textarea { background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 10px; padding: 9px 12px; color: var(--text); font: inherit; resize: vertical; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .ch { border: 1px solid var(--line-2); background: transparent; color: var(--muted); border-radius: 999px; padding: 6px 13px; font-weight: 600; font-size: 13.5px; }
  .ch:hover { color: var(--text); border-color: var(--gold-dim); }
  .ch[aria-pressed="true"] { background: rgba(227, 194, 112, 0.16); color: var(--gold-glow); border-color: var(--gold); }
  .foot { display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; }
  .link { border: 0; background: none; padding: 0; font-weight: 600; }
  .link.danger { color: var(--dim); }
  .link.danger:hover { color: var(--bad); }
  @media (max-width: 720px) {
    .sts { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
  @media (max-width: 640px) {
    .q { grid-template-columns: 26px minmax(0, 1fr) 28px 46px; }
    .q input[type="range"] { grid-column: 1 / -1; grid-row: 2; }
  }
</style>
