# Theming

The NovaIDE UI follows an 8pt spacing grid with Material-adjacent surfaces. Three themes ship with v0.1:

- **Light** – optimised for bright environments.
- **Dark** – default appearance matching the system colour scheme.
- **Solarized** – high-contrast palette defined in both the Svelte UI and Monaco editor.

Themes are applied using CSS custom properties inside the desktop layout. Monaco editor themes are registered dynamically to keep the code editor in sync with the outer shell. Extensions can contribute additional themes by emitting a `theme:register` event with a palette description.
