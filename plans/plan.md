# Vello Canvas Rail MVP Plan (Merged)

## Vision & Scope
- Ship a standalone "canvas rail" runtime: Rust → Wasm component apps render through a host-owned Vello surface with real input handling.
- Treat HTML/Blitz and Vello/Wasm as peer entrypoints; this prototype focuses solely on the Vello rail so we can validate the vertical slice end-to-end.
- Deliver a counter app demonstrating lifecycle, draw calls, pointer & keyboard events, redraw scheduling, and graceful failure handling.

```
┌───────────────────────────────────┐
│  Guest Wasm Component             │
│  - State/update/draw loop         │
│  - Uses typed host caps (WIT)     │
└──────────────┬────────────────────┘
               │ component model ABI
┌──────────────┴────────────────────┐
│  Host Runtime                     │
│  - winit event loop               │
│  - wgpu + Vello renderer          │
│  - Wasmtime component runtime     │
│  - Schedules frames & input       │
└───────────────────────────────────┘
```

## Guiding Principles
- **Typed boundary**: Use WIT + Wasm component model to keep the ABI small, versionable, and language-friendly.
- **Immediate-mode draw contract**: Guest issues draw primitives during a frame callback; host collects them into a Vello scene, flushes after callback returns.
- **Host authority**: Host owns GPU/device scale, manages frame pacing, coalesces redraw requests, and sandbox guest failures.
- **Simplicity first**: One window, one canvas, one guest instance. No Blitz integration, networking, or advanced text.

## MVP End State
- `frontier-wasm` binary accepts `--component /path/app.wasm`, opens a window, and renders the guest-driven scene via Vello.
- Counter component compiles with `cargo component`, maintains local state (no `static mut`), renders background, +/- controls, current count; responds to pointer and keyboard (`+`, `-`, `Space`, `Enter`).
- Resize + HiDPI handled: host passes logical size & scale; guest re-lays out accordingly.
- Guest traps or panics surface as an overlay with option to restart without killing the host.

## WIT Contract (Draft v0.1.0)
```wit
package vello:canvas@0.1.0

/// Shared math helpers
interface math {
    record vec2 { x: float32, y: float32 }
    record color { r: float32, g: float32, b: float32, a: float32 }
}

/// Host capabilities the guest can call.
interface host {
    use math.{color, vec2}

    /// Clear the current scene background (call once per frame before drawing).
    clear: func(c: color)

    /// Fill axis-aligned rectangle specified by top-left + size (logical pixels).
    fill-rect: func(origin: vec2, size: vec2, color: color)

    /// Draw text anchored at baseline origin using a bundled font.
    draw-text: func(text: string, origin: vec2, size: float32, color: color)

    /// Request another animation frame; host coalesces multiple calls.
    request-frame: func()

    /// Debug logging surfaced via host console.
    log: func(level: enum { trace, debug, info, warn, error }, message: string)
}

/// Events/lifecycle callbacks the guest exports.
interface app {
    use math.vec2

    record logical_size { width: float32, height: float32, scale_factor: float32 }

    record pointer_button { primary: bool, secondary: bool }

    record modifiers {
        shift: bool,
        ctrl: bool,
        alt: bool,
        meta: bool,
    }

    enum pointer_kind { mouse, touch, pen }

    record pointer_event {
        kind: pointer_kind,
        position: vec2,
        buttons: pointer_button,
        modifiers: modifiers,
        pointer_id: u64,
    }

    record key_event {
        key: string,        // UTF-8 text or key identifier (e.g. "ArrowUp")
        code: string,       // Physical key code (e.g. "KeyK")
        modifiers: modifiers,
        is_repeat: bool,
    }

    /// Called once after component instantiation.
    init: func(initial: logical_size)

    /// Called when window logical size or scale factor changes.
    resize: func(new: logical_size)

    /// Pointer events targeting the canvas.
    pointer-down: func(evt: pointer_event)
    pointer-up: func(evt: pointer_event)
    pointer-move: func(evt: pointer_event)

    /// Keyboard focus is owned by the host; key events delivered when focused.
    key-down: func(evt: key_event)
    key-up: func(evt: key_event)

    /// Frame callback. Host only invokes when guest requested redraw.
    frame: func(dt_ms: float32)
}

world canvas-app {
    import host
    export app
}
```

## Architecture Details
- **Frame orchestration**: Host zeroes a per-frame command buffer, invokes guest `frame()`, tracks all `host::*` imports executed, then submits the accumulated primitives to Vello before presenting. Out-of-frame drawing calls log warnings.
- **Input routing**: winit events converted to logical coordinates; pointer id stable per contact; modifiers derived from winit `ModifiersState`. Keyboard `key` uses text content when available, fallback to key name.
- **State safety**: Guest bindings provide a `Component` struct; counter stores state in `RefCell<State>` or similar to avoid `static mut`.
- **Text**: Bundle single font (e.g., Noto Sans) in host; expose limited alignment (baseline). Document limitation in README.
- **Error handling**: Wrap guest calls, intercept traps; display overlay with error message and `R` to restart.

## Implementation Phases
1. **Host Rendering Scaffold**
   - Create `crates/host` (e.g., `frontier-wasm-host`) with winit window, wgpu initialization, Vello renderer drawing static rectangle + text.
   - Validate on macOS/Linux; ensure logical vs physical size conversion works.

2. **WIT & Bindings Setup**
   - Place WIT package under `wit/vello/canvas.wit`; version it (`0.1.0`).
   - Integrate `wit-bindgen` for host and guest; check generated code into repo.
   - Document ABI in `docs/`.

3. **Wasmtime Component Integration**
   - Add Wasmtime (preview2) to host; load `canvas-app` world; implement host imports.
   - Manage instance lifecycle (instantiate on startup, allow restart on failure).

4. **Command Buffer & Draw Pipeline**
   - Implement per-frame command accumulation; map to Vello scene builder.
   - Optimize by reusing buffers and ensuring `request-frame` throttles to avoid spin.
   - Add simple text rendering via Vello’s glyph cache with bundled font asset.

5. **Input Plumbing**
   - Translate winit pointer/keyboard events to WIT structs, including modifiers and pointer ids.
   - Handle focus (click to focus, `Esc` to blur) to control keyboard delivery.

6. **Guest Counter Component**
   - Create `crates/counter-component` using `cargo component`.
   - Implement state/update/render using generated bindings, safe Rust patterns.
   - Build instructions (Just recipe) producing `.wasm` in `target/wasm32-wasip1/release/`.

7. **Host ↔ Guest Integration Testing**
   - Wire host to call guest `init`, deliver events, and render output.
   - Add error overlay + restart; confirm redraw scheduling works.
   - Manual smoke command `just demo-counter`.

8. **Docs & Notes**
   - Update repository README with build/run steps, known limitations.
   - Capture follow-up tasks (multi-surface, Blitz embedding) in `notes/`.

## Open Questions
- Headless/CI: Do we need surfaceless WGPU backend for automated tests now or can we defer?
- Font distribution: ship bundled font vs. system default selection.
- Immediate vs retained scenes: MVP stays immediate, but we should track the need for retained handles when perf matters.
- Runtime choice: Wasmtime is default; evaluate `wasmi` if footprint becomes an issue.

## Risks & Mitigations
- **ABI churn**: Version WIT (`0.1.0`) and avoid breaking fields unless absolutely necessary; document change process.
- **Event fidelity**: Expand pointer/keyboard structs now so we don’t rev for basic features (modifiers, multi-touch).
- **Performance**: Limit allocations by reusing buffers, coalescing `request-frame`, and avoiding heavy logging in frame loops.
- **Platform coverage**: Exercise macOS and Linux early; create tracking item for Windows support.

## Follow-Ups (Post-MVP)
- Multiple canvases/app instances per window.
- `<vello-canvas>` element for embedding inside Blitz layout once rail stabilizes.
- Resource handles (images, gradients), richer text (Parley), a11y via AccessKit.
- Packaging & manifest format for distributing components, including entrypoint selection.
- Optional DOM interop capabilities for hybrid experiences.
