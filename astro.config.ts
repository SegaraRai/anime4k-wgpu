import preact from "@astrojs/preact";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

// https://astro.build/config
export default defineConfig({
  srcDir: "./web-demo-src",
  integrations: [preact()],
  vite: {
    plugins: [tailwindcss()],
  },
});
