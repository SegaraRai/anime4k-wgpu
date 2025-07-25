import preact from "@astrojs/preact";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";
import macros from "unplugin-macros/vite";

// https://astro.build/config
export default defineConfig({
  site: process.env.SITE_URL ?? "https://anime4k-wgpu.roundtrip.dev",
  srcDir: "./web-demo-src",
  integrations: [preact()],
  vite: {
    plugins: [tailwindcss(), macros()],
  },
});
