<script lang="ts">
  import Icon from "../Icon.svelte";
  import { toast } from "../toast.svelte";
</script>

<div class="toast" class:show={toast.visible} role="status" aria-live="polite">
  <Icon name={toast.icon} />
  <span>{toast.text}</span>
</div>

<style>
  .toast {
    position: fixed; top: 16px; right: 16px; z-index: 150;
    display: flex; align-items: center; gap: 10px;
    max-width: calc(100% - 32px);
    padding: 10px 14px; border-radius: 12px;
    background: rgba(12, 14, 17, 0.92);
    border: 1px solid rgba(96, 216, 216, 0.45);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5), 0 0 20px rgba(96, 216, 216, 0.12);
    opacity: 0; transform: translateY(-12px);
    transition: opacity 0.2s ease, transform 0.2s ease;
    pointer-events: none;
  }
  .toast.show { opacity: 1; transform: none; }
  .toast :global(svg) { color: var(--neon); }
</style>
