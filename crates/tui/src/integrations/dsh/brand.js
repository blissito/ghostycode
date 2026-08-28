/* ghosty-brand: an explicit identity lockup for the DSH skin.
 *
 * DSH renders this component through its additive shell.overlay Slot. The
 * lockup is pointer-inert, compact on narrow viewports, uses the active skin
 * tokens, and leaves DSH-owned chrome untouched.
 */
function GhostyBrand() {
	var css =
		"#ghosty-brand-lockup{" +
		"position:fixed;top:18px;right:20px;z-index:40;display:flex;align-items:center;gap:10px;min-width:272px;padding:10px 12px;box-sizing:border-box;pointer-events:none;user-select:none;border:1px solid var(--dsw-alias-brand-primary,#6aaef2);border-left:3px solid var(--dsw-alias-state-business-primary,#f6c453);border-radius:12px;background:var(--dsw-alias-bg-layer-1,#0e1729);color:var(--dsw-alias-label-primary,#f6f2e8);box-shadow:0 18px 44px rgba(0,0,0,.38),inset 0 1px 0 rgba(255,255,255,.07);font-family:Inter,ui-sans-serif,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;line-height:1;" +
		"}" +
		"#ghosty-brand-mark{" +
		"display:grid;place-items:center;width:34px;height:34px;flex:0 0 auto;border-radius:10px;background:#03070d;box-shadow:0 8px 22px rgba(0,0,0,.42);" +
		"}" +
		"#ghosty-brand-mark svg{display:block;width:24px;height:24px;}" +
		"#ghosty-brand-copy{display:flex;min-width:0;flex:1;flex-direction:column;gap:5px;}" +
		"#ghosty-brand-kicker{display:block;color:var(--dsw-alias-state-business-primary,#f6c453);font-size:9px;font-weight:800;letter-spacing:2.1px;white-space:nowrap;}" +
		"#ghosty-brand-name{display:block;color:var(--dsw-alias-label-primary,#f6f2e8);font-size:15px;font-weight:850;letter-spacing:1.7px;white-space:nowrap;}" +
		"#ghosty-brand-bridge{display:block;margin-left:auto;color:var(--dsw-alias-brand-primary,#6aaef2);font-family:ui-monospace,\"SFMono-Regular\",Menlo,Consolas,monospace;font-size:9px;font-weight:750;letter-spacing:.8px;white-space:nowrap;}" +
		"@media(max-width:759px){" +
		"#ghosty-brand-lockup{top:8px;right:8px;min-width:0;padding:7px 9px;gap:7px;}" +
		"#ghosty-brand-mark{width:28px;height:28px;}" +
		"#ghosty-brand-mark svg{width:19px;height:19px;}" +
		"#ghosty-brand-kicker,#ghosty-brand-bridge{display:none;}" +
		"}";

	return React.createElement(
		React.Fragment,
		null,
		React.createElement("style", { "data-ghosty-brand-style": true }, css),
		React.createElement(
			"aside",
			{
				id: "ghosty-brand-lockup",
				"aria-label": "Whale Brothers. Ghosty connected to DeepSeek Harness.",
			},
			React.createElement(
				"span",
				{ id: "ghosty-brand-mark", "aria-hidden": true },
				// Signal Current: the two-color contract mark on ink (design system
			// BRIEF.md). No emoji — the identity is a real inline SVG.
			React.createElement(
				"svg",
				{ viewBox: "0 0 64 64", width: "24", height: "24" },
				React.createElement("path", {
					fill: "#f6c453",
					d: "M7 57c9-13 21-15 25-25 3-7-1-12-10-16 6-1 11 1 14 6 3-6 10-11 19-13-1 10-5 17-12 21-4 3-5 8-8 14-5 9-15 14-28 13Z",
				}),
				React.createElement("path", {
					fill: "#48d7ff",
					d: "M28 58c10-8 15-15 17-26 4 8 2 18-3 26H28Z",
				}),
			),
			),
			React.createElement(
				"span",
				{ id: "ghosty-brand-copy" },
				React.createElement("span", { id: "ghosty-brand-kicker" }, "WHALE BROTHERS"),
				React.createElement("span", { id: "ghosty-brand-name" }, "GHOSTY"),
			),
			React.createElement("span", { id: "ghosty-brand-bridge" }, "\u00d7 DEEPSEEK HARNESS"),
		),
	);
}
