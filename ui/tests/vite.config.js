import { mergeConfig } from "vite";
import base from "../vite.config.js";
export default mergeConfig(base, {
  build: { outDir: "dist/fixture", rollupOptions: { input: "tests/ipc-fixture.html" } },
});
