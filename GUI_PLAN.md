# Website-Searcher GUI (Tauri + React) Plan

## Executive Summary
A cross-platform desktop GUI will be added alongside the existing CLI/TUI.  It will be implemented with **Tauri** (Rust backend, system WebView frontend) and **React** (TypeScript) for a dark-theme-only interface.  Two binaries will be produced:
1. `websearcher` – existing CLI/TUI (unchanged).
2. `websearcher-gui` – new desktop app that re-uses all core logic compiled into a shared Rust **library crate**.

The initial release targets Windows (dev focus) and Linux; macOS artifacts are optional but automated via CI.  MSI installers will be generated for Windows, and zipped binaries for other OSes.

## Research Findings
- **Tauri** is the de-facto lightweight Rust GUI framework with secure IPC, small bundle sizes (<10 MB with React), and first-party GitHub Action (`tauri-action`) support for building installers.
- Tauri’s **`tauri::test` mock runtime** allows Rust-side unit/integration tests without spawning a real window.
- **WebDriver / `tauri-driver`** enables one end-to-end smoke test per OS in CI.
- Front-end testing best practice: **React Testing Library + Vitest/Jest** for component logic; **axe-core** can provide optional a11y checks.
- Windows MSI creation is handled by `tauri-bundler`; DMG generation for macOS can run headless in GitHub-hosted macOS runners – no local Apple hardware required.

## Objectives & Goals
- Deliver a dark-mode-only GUI with:
  • Top-center search bar plus collapsible site checklist (defaults to All)
  • Checkboxes for CF solver, Playwright, limit, etc.
  • Results table pane (right) mirroring TUI layout; rows open URLs in default browser
  • Recent-searches (persisted locally) quick-access sidebar
- Keep CLI/TUI binary intact; share core logic via library crate
- Provide MSI + zip artifacts in GitHub Releases
- Maintain full test coverage (unit + integration + e2e)
- Minimal CI runtime increase (<20%)

## Methodology / Approach
1. **Restructure crates**
   1. Move current `src/` into `crates/core/` → new library crate `website_searcher_core`
   2. Keep CLI in `crates/cli/` (binary `websearcher`) depending on `core`
   3. Generate `src-tauri/` via `tauri init`, and inside point `Cargo.toml` to `core` with `path = "../core"`
2. **Frontend**
   - React + TypeScript scaffold (`pnpm create vite@latest` inside `src-tauri/../gui`)
   - Tailwind CSS (dark theme default) + DaisyUI or Radix-UI components
3. **IPC layer**
   - Expose Rust commands (`#[tauri::command]`) wrapping existing library functions: `search(query, sites, flags) -> Vec<SearchResult>`
4. **State management**
   - React Context for options; localStorage for recent searches
5. **Packaging**
   - Configure `tauri.conf.json` for MSI (Windows) and AppImage/zip (Linux); DMG left enabled but optional
6. **Versioning**
   - GUI shares semver with CLI; release tag drives both artifact pipelines

## Timeline & Milestones
| Date (wk) | Milestone |
|-----------|-----------|
| W1 | Crate split & core library compile OK, CLI unaffected |
| W2 | Tauri scaffold builds empty window on Win & Linux |
| W3 | Implement React UI skeleton; search executes & renders results |
| W4 | Recent-searches & collapsible sites implemented; dark-theme styling complete |
| W5 | Unit tests (Rust + front-end) passing; basic e2e smoke test green |
| W6 | CI extended (`tauri-action`, MSI produced); first pre-release tag |

## Resource Requirements
- Rust stable (same toolchain) + `tauri-cli`
- Node 20 + pnpm for frontend
- GitHub runners: ubuntu-latest, windows-latest, macos-13 (Intel) / macos-14 (ARM) for packaging
- Estimated engineer effort: 4-6 weeks 1 FTE

## Risk Assessment
| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|-----------|
| WebView differences (Win WebView2 vs Linux GTK) | UI glitches | Med | test on each OS in CI; use Tauri recommended APIs |
| Increased CI time | Slower PR feedback | Low | cache npm + Rust; restrict GUI e2e to one smoke test |
| Bundle size creep | Large downloads | Low | audit deps; use Tauri `multipart` optimization |
| Code-signing certificates | Needed for SmartScreen | Low-Med | leave unsigned for OSS; document signing later |

## Implementation Strategy
### Phase 1 – Crate Split ✅ (Complete)
- `cargo new --lib crates/core` then move logic files; update `mod` paths
- Update paths in tests; run full suite

### Phase 2 – GUI Scaffolding ✅ (Complete)
- `cargo install tauri-cli` ; `pnpm create vite` ; `tauri init --ci`
- Configure `tauri.conf.json` → dark mode only (`preferredTheme: "dark"`)
- Add React router layout: SearchBar, OptionsDrawer, ResultsTable

### Phase 3 – Command Wiring ✅ (Complete)

### Phase 4 – Tests
1. **Rust unit/integration**: unchanged for core; add tests under `src-tauri/tests` using `tauri::test` for command handlers
2. **Frontend unit**: React Testing Library + Vitest
3. **End-to-End**: single Playwright-with-Tauri smoke across OS (launch app, type query, assert table row appears)

### Phase 5 – CI Updates
- Extend `ci.yml` with job `build-gui` after `build-test`
```yaml
- name: Build Tauri GUI
  uses: tauri-apps/tauri-action@v0
  with:
    args: "--ci --target ${{ matrix.os }}"
```
- Upload MSI/zip as artifacts on `push tags` workflow

## Monitoring & Evaluation
- KPIs: GUI build passes CI; MSI < 30 MB; search returns identical results count as CLI for same inputs
- Review metrics each release; collect user feedback on GitHub

## Next Steps
1. Start W5 and Phase 4 by beginning to develop new tests
2. Finish tests (and W5)
3. Start W6 and phase 5 by beginning the CI updates
4. finish W6 
