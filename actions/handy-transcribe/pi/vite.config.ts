import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  base: "./",
  plugins: [svelte()],
  build: { outDir: "../plugin/io.github.mario.handytranscribe.sdPlugin/pi", emptyOutDir: true },
});
