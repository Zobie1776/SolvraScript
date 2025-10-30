# Solvra GUI Desktop Design Overview

This document captures the design goals and modular plan for the Solvra GUI desktop surface. The objective is to deliver a smooth, reliable, multi-window experience that integrates launcher panels, task controls, and Solvra applications (IDE, CLI, App Store) while remaining theme-aware and responsive.

## Goals
- **Single entry point:** Build the initial desktop experience within `crates/shell_launcher` (renamed internally to `DesktopApp`).
- **Task & Window Management:** Provide a task bar with application shortcuts, active window indicators, and basic window controls (open, focus, close/minimise).
- **Panels:** Include quick-access panels for Solvra IDE, CLI, App Store, and a system status area.
- **Responsive Layout:** Adjust layout spacing, typography, and panel arrangement based on window dimensions.
- **Dynamic Themeing:** Consume `theme_engine` tokens (colors, typography, effects) to style UI elements with sharp angles and high contrast “Solvra” aesthetic.
- **Extensibility:** Architectural hooks for IPC integration with the compositor and future driver-backed rendering.

## Architecture Plan
1. **State Model (`DesktopState`)**
   - `ThemeTokens` for colors/effects.
   - `Vec<WindowEntry>` representing open windows.
   - `TaskBarState` storing pinned app definitions, active selection, and clock data.
   - `PanelsState` toggling global panels (notifications, quick settings).

2. **Message Enum (`Message`)**
   - User interactions: launch app, close/minimise window, focus window, theme refresh, tick events.
   - Responsive events: window resized, layout breakpoints toggled.

3. **View Composition**
   - Root layout uses `Column` (desktop surface) and `Container` backgrounds.
   - `pane_grid` to render window tiles with draggable handles (simulated multi-window control).
   - Bottom `TaskBar` (`Row` with buttons, status labels, slanted fonts).
   - Accent gradients derived from theme colors.

4. **Styling Helpers**
   - `StyleCatalog` mapping tokens to consistent iced styles (buttons, containers, text).
   - Utility for slanted/italic text, neon glows, and drop shadows.

5. **Responsiveness**
   - Breakpoints (compact vs. spacious). Layout adjustments for spacing, icon size, text scale.
   - Auto-hide taskbar labels when width is constrained.

6. **Future Hooks (Not Implemented Yet)**
   - IPC to compositor for real window management.
   - Real-time system status integration (battery/network).
   - Plugin pipeline for third-party panels.

## Implementation Steps
1. Replace `crates/shell_launcher/src/main.rs` with a full-featured `DesktopApp` using iced.
2. Add supporting structs/enums within the same file (modular sections per Zobie format).
3. Provide unit tests (where feasible) focusing on state transitions (open/close windows, taskbar updates).
4. Document the module with inline comments and update README sections after functionality stabilises.

Completion of this step establishes the core UI scaffold. Subsequent iterations will connect the compositor, apply animations, and expand panel functionality.
