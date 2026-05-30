import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: "jsdom",
    globals: true,
    include: ["src/**/*.{test,spec}.ts"],
    coverage: {
      provider: "v8",
      // Keine künstlichen Ausschlüsse: gemessen wird, was die Tests tatsächlich
      // anfassen (App.vue, wow.ts). Der Vue-Bootstrap main.ts wird von keinem
      // Test importiert und daher — wie ein Entry-Point — nicht gewertet.
      reporter: ["text", "html"],
      thresholds: {
        lines: 95,
        functions: 95,
        statements: 95,
        branches: 90,
      },
    },
  },
});
