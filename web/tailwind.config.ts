import type { Config } from "tailwindcss";

export default {
  content: ["./app/**/*.{ts,tsx}", "./components/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        // The surface and ink tokens resolve through CSS custom properties so
        // docs can still re-theme, while the shared accent palette stays oceanic.
        paper: "rgb(var(--c-paper) / <alpha-value>)",
        "paper-deep": "rgb(var(--c-paper-deep) / <alpha-value>)",
        "paper-edge": "rgb(var(--c-paper-edge) / <alpha-value>)",
        "paper-card": "var(--paper-card)",
        "paper-line": "#1B2230",
        "paper-line-soft": "#CBD3DF",
        ink: "rgb(var(--c-ink) / <alpha-value>)",
        "ink-soft": "rgb(var(--c-ink-soft) / <alpha-value>)",
        "ink-mute": "rgb(var(--c-ink-mute) / <alpha-value>)",
        indigo: "#315FD8",
        "indigo-deep": "#2448A6",
        "indigo-pale": "#E8EEF8",
        ochre: "#7A5500",
        jade: "#08766D",
        cobalt: "#315FD8",
      },
      fontFamily: {
        // The real display face is Fraunces, loaded by next/font in
        // app/[locale]/layout.tsx onto --font-display (globals.css .font-display
        // resolves the same stack). Space Grotesk was never loaded — naming it
        // here only produced a utility that silently fell back to sans-serif.
        display: ["var(--font-display)", '"Fraunces"', '"Noto Serif SC"', "Georgia", "serif"],
        body: ["var(--font-body)", '"IBM Plex Sans"', '"Noto Sans SC"', "ui-sans-serif", "system-ui", "sans-serif"],
        cjk: ["var(--font-cjk)", '"Noto Serif SC"', '"Source Han Serif SC"', "serif"],
        mono: ["var(--font-mono)", '"JetBrains Mono"', "ui-monospace", "Menlo", "monospace"],
      },
      letterSpacing: {
        crisp: "-0.018em",
        wider: "0.08em",
        widest: "0.18em",
      },
    },
  },
  plugins: [],
} satisfies Config;
