import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri geliştirme sunucusu sabit portta çalışır; hata ekranı Tauri'nin kendi
// hatalarını gizlemesin diye clearScreen kapalı.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "es2022",
    // Windows'ta WebView2 (Chromium) kullanıldığı için modern çıktı yeterli.
    minify: "esbuild",
    sourcemap: false,
  },
});
