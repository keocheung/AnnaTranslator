# Repository Guidelines

## Project Structure & Module Organization

- `src/`: Vue 3 + TypeScript frontend (UI, i18n, settings, history views).
- `src/locales/`: Translation strings (`en.ts`, `zh-CN.ts`).
- `src-tauri/`: Tauri Rust backend (HTTP server, clipboard, cache, history).
- `script/`: Helper scripts for Ren'Py/RPGMaker integration.
- `dist/`: Frontend build output (generated).
- `index.html`, `vite.config.ts`, `tsconfig.json`, `deno.json`: app entry and tooling config.

## Build, Test, and Development Commands

- `deno task dev`: Run Vite dev server for the frontend (defaults to `http://localhost:5173`).
- `deno task build`: Build the frontend bundle into `dist/`.
- `deno task preview`: Serve the built frontend for quick checks.
- `deno task lint`: Type-check Vue/TS with `vue-tsc` (no emit).
- `deno task format`: Format all files with Prettier.
- `deno task tauri dev`: Run the full Tauri app in dev mode.
- `deno task tauri build`: Build the desktop app packages.

## Coding Style & Naming Conventions

- Formatting is handled by Prettier; prefer double quotes in TS/JS.
- Use 2-space indentation in frontend files to match existing formatting.
- Vue SFCs use `PascalCase` filenames (e.g., `SettingsWindow.vue`).
- Keep Tauri Rust modules small and focused (`src-tauri/src/*.rs`).

## Testing Guidelines

- No dedicated unit test framework is configured yet.
- Use `deno task lint` as the primary CI-friendly check.
- If you add tests, document the runner and naming convention here.

## Commit & Pull Request Guidelines

- Commit messages follow `type: subject` (e.g., `build: update rust deps`, `doc: update README`, `release: 0.2.0`).
- Keep subjects concise, present tense, and scoped when useful (`httpserver: ...`).
- PRs should include: a short summary, key changes, how to verify, and screenshots for UI changes.

## Configuration & Tips

- Default HTTP port is `17889`; override with `TRANSLATOR_PORT`.
- OpenAI settings are configured in-app; defaults are documented in `README.md`.
