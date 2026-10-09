import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

export default defineConfig({
  resolve: { alias: { "@desktop": fileURLToPath(new URL("../desktop", import.meta.url)), "@": fileURLToPath(new URL(".", import.meta.url)) } },
});
