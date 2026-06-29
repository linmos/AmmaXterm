import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  build: {
    // Minify with terser, not esbuild. xterm 6.0.0 ships its `const enum` (e.g.
    // the DECRPM states in `requestMode`) as a runtime enum, and esbuild's
    // minifier drops the enum holder's `var` declaration, emitting
    // `void 0 || (i = {})` where `i` is never declared. ES modules are always in
    // strict mode, so that throws `ReferenceError: i is not defined` the first
    // time xterm answers a DECRQM query (which vi/vim/tmux/less send on startup),
    // crashing xterm's write loop and freezing the whole terminal. Only the
    // packaged build minifies, which is why `tauri dev` looked fine. Terser
    // minifies the same code correctly. The JSDoc cast keeps checkJs happy: a
    // bare "terser" widens to `string` and fails Vite's literal-union type.
    minify: /** @type {'terser'} */ ('terser')
  }
}));
