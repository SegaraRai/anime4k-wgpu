import preact from "@astrojs/preact";
import { defineConfig } from "astro/config";

import tailwindcss from "@tailwindcss/vite";

// https://astro.build/config
export default defineConfig({
  srcDir: "./web-demo-src",
  integrations: [preact()],

  vite: {
    plugins: [tailwindcss()],
  },
});