# PRD: Electron WebGPU Desktop Shell Experiment

## Goal
Create a branch-isolated experimental desktop shell that uses Electron instead of Tauri and enables Chromium startup flags for Vulkan and unsafe WebGPU. The experiment should keep the existing React/Vite workbench usable and preserve the fake-stream debug path for local verification.

## Requirements
- Add an Electron-based desktop app path without deleting the existing Tauri path.
- Launch Chromium with `--enable-features=Vulkan` and `--enable-unsafe-webgpu` before app ready.
- Reuse the existing UI where possible instead of forking the frontend.
- Make the workbench usable in Electron, including bootstrap, fake session autostart, and the core desktop command/event bridge needed by the current UI.
- Keep desktop integration behind a frontend adapter boundary so the UI is not tightly coupled to Tauri-specific imports.
- Provide a practical development command or script for running the Electron experiment.
- Preserve current Tauri flow so the experiment remains reversible.

## Acceptance Criteria
- A developer can start the Electron experiment on the feature branch and open the workbench window.
- The Electron main process appends the required Chromium flags before window creation.
- The frontend no longer directly depends on Tauri-only imports in app entry/store wiring; a desktop adapter abstraction exists.
- The Electron path can bootstrap the app state and keep the fake stream debug workflow usable.
- The codebase still builds for the frontend, and the experimental desktop path is documented enough for local verification.

## Non-Goals
- Removing Tauri from the repository.
- Perfect packaging parity with the existing Tauri bundle on day one.
- Re-architecting the Rust runtime beyond what is necessary to make the Electron experiment usable.
