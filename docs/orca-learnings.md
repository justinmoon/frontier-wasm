# Orca Learnings

Synthesis of the prior Orca notes (`docs/orca-learnings-claude.md`) with additional exploration of the Orca repository (`~/code/orca`) to inform the evolution of frontier-wasm.

## Quick Snapshot
- Orca ships a Wasm-native runtime that bundles a stock host executable with guest modules and assets, producing self-contained `.app`/`.exe` outputs via `orca bundle` (see `src/tool/bundle.c`).
- It targets macOS 14+ and Windows 10+, with language tooling centered on C today (Odin/Zig in flight), powered by a wasm3 interpreter plus an in-progress in-house VM (`src/warm`).
- The SDK layers canvas rendering, GLES/WebGPU backends, UI widgets, capability-scoped storage, and explicit font/text primitives, giving app authors batteries-included ergonomics.

## Architectural Notes
### Runtime & ABI
- Apps export optional event handlers (`oc_on_init`, `oc_on_frame_refresh`, `oc_on_mouse_down`, etc.) that the host invokes. This mirrors our WIT callbacks but proves the pattern scales with real apps.
- The interface surface is defined in a large custom `api.json` that feeds C headers and docs. It works but is bespoke; our WIT/component-model path remains a strategic advantage for multi-language expansion.
- Orca’s wasm3 baseline keeps implementation simpler but trades off performance and modern Wasm features. Frontier’s Wasmtime choice is heavier to embed yet gives immediate perf headroom and component model support.

### Graphics Stack
- Multiple render paths coexist: a 2D vector canvas, OpenGL ES 3.1, WebGPU, and Metal-specific shims. Samples (`samples/clock`, `samples/fluid`, `samples/triangle`, `samples/ui`) showcase how this enables both UI and simulation workloads.
- The canvas API spans low-level path construction, transform stacks, image drawing, and text outlining. In contrast, our current WIT surface emphasises high-level shapes and text helpers; matching Orca’s flexibility will likely require adding path/transform/image primitives around Vello.

### UI, Text, and Accessibility
- Orca’s host-side UI library (under `src/ui/`) provides retained widget state and host-managed text editing (`oc_ui_text_box_info`) so apps do not reimplement IME, selection, or clipboard logic.
- Accessibility is still early: there is no deep AccessKit-equivalent yet, offering a chance for Frontier to differentiate by baking semantics into the runtime contract rather than per-app opt-in.
- Fonts are bundled per app and loaded explicitly with Unicode range selection, enabling consistent typography regardless of platform fonts. We currently expose a single host font; adding font registration APIs while keeping good defaults will help parity.

### Distribution & Tooling
- The `orca` CLI handles SDK path discovery, bundling, icon generation (macOS `iconutil`/`sips`, Windows resource injection), and resource copying. This “one command → shippable bundle” developer experience is a key part of the polish.
- Orca lacks Linux support today and distributes SDK tarballs manually. Frontier can lean on Nix/just tooling plus cross-platform support (including Linux) to stand out, but we should present equally cohesive bundling flows.

## Implications for Frontier-Wasm
- **Packaging:** We should evolve `just dist` into a first-class `frontier bundle` (or similar) that emits signed `.app`, `.msix`/`.exe`, and eventually Linux packages, embedding Wasmtime, Vello shaders, and assets with minimal configuration.
- **Rendering API:** Pair our simple helpers with an optional advanced layer exposing retained Vello scenes, path ops, transforms, gradients, and images. This satisfies both “get started quickly” and “port existing engines” audiences.
- **Framework Adapters:** Orca’s C focus means each language needs custom bindings. By leaning on WIT and the component model we can build adapters for Rust (egui, gpui, iced), Go, and C++ more systematically, but we must still invest in per-framework shims to translate their painting calls into Frontier scenes.
- **Text & Widgets:** Adopt the host-managed text input approach (backed by Parley) and ship reusable widget scaffolding so developers get great IME/selection behavior out of the box. Long term we can layer richer frameworks (Xilem) on top.
- **Accessibility:** Formalise semantics in the guest API and map them to AccessKit from day one. Orca doesn’t cover this yet, so strong accessibility can be a headline differentiator.
- **Graphics Backends:** Vello-first remains our identity, but we should roadmap interop layers for GLES/WebGPU to unlock simulation/game workloads when demand appears.

## Gaps & Opportunities Compared to Orca
- **Performance:** Orca’s wasm3 interpreter is slower; Wasmtime’s JIT plus component model support is a competitive edge we should emphasise.
- **Platform Reach:** We can target Linux and, longer term, web deployments; Orca is desktop-only right now.
- **Documentation:** Orca’s docs (Quick Start, API cheatsheets) are comprehensive. Replicating that level of guides, samples, and reference docs will be essential as we grow our API surface.
- **Ecosystem:** Orca validates interest in Wasm-native runtimes. Collaboration or knowledge-sharing (e.g., around bundling, UI ergonomics) could accelerate both projects while letting Frontier concentrate on accessibility, text quality, and multi-language adapters.

## Suggested Next Steps
1. **Design a Frontier bundling manifest** capturing metadata, icons, resources, and capability requests; prototype a CLI command that produces macOS/Windows bundles akin to `orca bundle`.
2. **Extend the rendering WIT** with path/transform/image primitives and evaluate how to map them onto Vello without regressing ease-of-use.
3. **Define host text-editing contracts** (selection, IME, clipboard) so Parley-backed widgets can ship as part of the runtime SDK.
4. **Outline an accessibility schema** that lets guests declare semantic nodes, then integrate AccessKit in the host to expose them to assistive tech.
5. **Audit documentation needs** (Quick Start, SDK reference, samples) to match Orca’s clarity and reduce learning friction for Frontier developers.
