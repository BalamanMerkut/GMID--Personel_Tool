<script lang="ts">
  let { checked, label, onchange, disabled = false }: { checked: boolean; label: string; onchange: (v: boolean) => void; disabled?: boolean } = $props();
</script>

<button class="switch" role="switch" aria-checked={checked} aria-label={label} {disabled} onclick={() => onchange(!checked)}>
  <i></i>
</button>

<style>
  .switch { border: 0; background: transparent; padding: 0; flex: none; }
  .switch:disabled { opacity: 0.5; cursor: default; }
  i {
    display: block; position: relative; width: 42px; height: 24px; border-radius: 999px;
    background: var(--coal); border: 1px solid var(--line-2); transition: background 0.2s, border-color 0.2s;
  }
  i::after {
    content: ""; position: absolute; top: 2px; left: 2px; width: 18px; height: 18px; border-radius: 50%;
    background: var(--muted); transition: transform 0.2s, background 0.2s;
  }
  [aria-checked="true"] i { background: rgba(227, 194, 112, 0.25); border-color: var(--gold); }
  [aria-checked="true"] i::after { transform: translateX(18px); background: var(--gold); }
</style>
