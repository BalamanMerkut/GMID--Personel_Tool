<script lang="ts">
  import { api, isVideo } from "../api";
  import { settings } from "../settings.svelte";

  /**
   * `active`: pencere görünür ve odakta mı. Değilse hareketli arka plan durur (denge kuralı).
   * `quality`: "light" iken bulanıklık ve hareket kapalı.
   */
  let { active, quality }: { active: boolean; quality: "high" | "light" } = $props();

  let video: HTMLVideoElement | undefined = $state();
  let gif: HTMLImageElement | undefined = $state();
  let still: HTMLCanvasElement | undefined = $state();

  const FOCUS = { center: "50% 50%", top: "50% 18%", bottom: "50% 82%", left: "18% 50%", right: "82% 50%" };

  const src = $derived(settings.bgKind === "file" && settings.bgPath ? api.fileSrc(settings.bgPath) : "");
  const kind = $derived(!src ? "none" : isVideo(settings.bgPath) ? "video" : /\.gif$/i.test(settings.bgPath) ? "gif" : "image");
  const moving = $derived(active && quality === "high");
  const blur = $derived(quality === "high" ? settings.bgBlur : 0);
  const pos = $derived(FOCUS[settings.bgFocus]);

  // Video: sadece görünürken oynar.
  $effect(() => {
    if (!video) return;
    if (moving) video.play().catch(() => {});
    else video.pause();
  });

  // GIF durdurulamadığı için ilk karesi bir tuvale çizilir; pencere arkadayken o gösterilir.
  function captureStill() {
    if (!gif || !still) return;
    still.width = gif.naturalWidth;
    still.height = gif.naturalHeight;
    still.getContext("2d")?.drawImage(gif, 0, 0);
  }
</script>

<div class="bg" aria-hidden="true">
  {#if settings.bgKind === "color"}
    <div class="fill" style:background={settings.bgColor}></div>
  {:else if kind === "video"}
    <video bind:this={video} {src} muted loop playsinline preload="auto" style:object-position={pos} style:filter={blur ? `blur(${blur}px)` : null} class:blurred={blur > 0}></video>
  {:else if kind === "gif"}
    <img bind:this={gif} {src} alt="" onload={captureStill} hidden={!moving} style:object-position={pos} style:filter={blur ? `blur(${blur}px)` : null} class:blurred={blur > 0} />
    <canvas bind:this={still} hidden={moving} style:object-position={pos} style:filter={blur ? `blur(${blur}px)` : null} class:blurred={blur > 0}></canvas>
  {:else if kind === "image"}
    <img {src} alt="" style:object-position={pos} style:filter={blur ? `blur(${blur}px)` : null} class:blurred={blur > 0} />
  {:else}
    <div class="mansion">
      <svg viewBox="0 0 520 300" class="castle">
        <path d="M10 290h500" />
        <path d="M40 290V120h34v22h22v-22h22v22h22v-22h34v170" />
        <path d="M150 290V170h220v120" />
        <path d="M150 170v-22h26v22M196 170v-22h26v22M298 170v-22h26v22M344 170v-22h26v22" />
        <path d="M212 170V86h96v84M200 86l60-66 60 66" />
        <path d="M240 290v-52a20 20 0 0 1 40 0v52" />
        <path d="M250 112h20v26h-20zM80 180h14v24H80zM426 180h14v24h-14z" />
        <path d="M372 290V120h34v22h22v-22h22v22h22v-22h34v170" />
      </svg>
    </div>
  {/if}
  {#if src}
    <div class="dim" style:background="rgba(10, 11, 13, {settings.bgDim / 100})"></div>
  {/if}
</div>

<style>
  .bg { position: fixed; inset: 0; z-index: 0; overflow: hidden; background: var(--night); }
  .fill, img, video, canvas, .dim { position: absolute; inset: 0; width: 100%; height: 100%; }
  img, video, canvas { object-fit: cover; }
  /* Bulanıklıkta kenarlar şeffaflaşmasın diye biraz büyüt */
  .blurred { transform: scale(1.06); }
  .mansion {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(1200px 600px at 85% -10%, rgba(227, 194, 112, 0.07), transparent 60%),
      radial-gradient(900px 700px at 100% 110%, rgba(11, 32, 62, 0.6), transparent 60%),
      var(--night);
  }
  .castle {
    position: absolute;
    right: -2%;
    bottom: -1%;
    width: min(62vw, 900px);
    fill: none;
    stroke: var(--gold);
    stroke-width: 1.4;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0.07;
  }
</style>
