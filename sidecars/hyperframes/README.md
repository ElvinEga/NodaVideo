# HyperFrames Sidecar Runtime

Vector motion graphics and AI-generated HTML scene sidecar for NodaVideo.

## Purpose

As defined in `AGENTS.md`:
> **HyperFrames**: retained for motion graphics and AI-generated HTML scenes only. It produces editable animated compositions; the Rust timeline owns their timing and placement. Never let it compete as a second timeline.

## Layout & Architecture

- `templates/`: Production JSON templates defining parameter schemas (`text`, `color`, `number`, `choice`, `boolean`) and HTML/SVG markup.
- `sidecars/hyperframes/renderer.js`: Headless CLI renderer invoked by `crates/hyperframes-bridge`.
- `sidecars/hyperframes/index.html`: Interactive preview testbed for developing new motion graphics.
