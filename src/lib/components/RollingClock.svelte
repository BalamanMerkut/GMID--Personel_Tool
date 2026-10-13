<script lang="ts">
  // Kayan rakamlar (kilometre sayacı gibi): değişen her rakam yenisi gelirken yukarı/aşağı kayar.
  // `dir` 1: sayı artıyor (yeni rakam alttan gelir), -1: azalıyor (üstten gelir).
  // `fast`: sondaki bu kadar karakter canlandırılmaz (salise gibi saniyede onlarca kez değişenler).
  import { cubicOut } from "svelte/easing";

  let { text, dir = 1, fast = 0 }: { text: string; dir?: 1 | -1; fast?: number } = $props();

  const chars = $derived([...text]);

  function roll(_node: Element, { from }: { from: number }) {
    return {
      duration: 300,
      easing: cubicOut,
      css: (t: number, u: number) => `transform: translateY(${(u * from * 100).toFixed(1)}%); opacity: ${t.toFixed(3)}`,
    };
  }
</script>

<span class="roll" aria-label={text}>
  {#each chars as ch, i (chars.length - i)}
    {#if ch >= "0" && ch <= "9" && i < chars.length - fast}
      <span class="cell" aria-hidden="true">
        {#key ch}<span class="d" in:roll={{ from: dir }} out:roll={{ from: -dir }}>{ch}</span>{/key}
      </span>
    {:else}
      <span aria-hidden="true">{ch}</span>
    {/if}
  {/each}
</span>

<style>
  .roll { display: inline-flex; align-items: center; font-variant-numeric: tabular-nums; line-height: 1.15; }
  /* Giden ve gelen rakam aynı hücrede üst üste; hücre taşanı keser, kenarlarda yumuşakça solar. */
  .cell {
    display: inline-grid; overflow: hidden; height: 1.15em;
    mask-image: linear-gradient(transparent, #000 18%, #000 82%, transparent);
  }
  .d { grid-area: 1 / 1; }
</style>
