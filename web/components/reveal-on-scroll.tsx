"use client";

/**
 * <RevealOnScroll> — the entrance system, mounted once on the homepage.
 *
 * It renders nothing. It finds the server's declarative marks and gives them
 * motion:
 *
 *   [data-reveal]         — one element settles in as it enters the viewport.
 *   [data-reveal-group]   — a container whose children settle in sequence
 *                           (a group that reads as a group), staggered by
 *                           `--reveal-delay` assigned here.
 *
 * The static page is the default and motion is progressive enhancement:
 * elements are only hidden ("armed") by this effect, never by the server
 * render, so no-JS, no-IntersectionObserver, and prefers-reduced-motion all
 * get the complete page with zero animation. Content already in view at load
 * is left alone — entrance motion is for what arrives, never for what the
 * reader is already looking at. The CSS side (globals.css, "Motion" section)
 * is gated on `no-preference` as a second, independent guard, and animates
 * transform and opacity only.
 */

import { useEffect } from "react";

const STAGGER_MS = 55;

export function RevealOnScroll() {
  useEffect(() => {
    if (!("IntersectionObserver" in window)) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    const targets: HTMLElement[] = Array.from(
      document.querySelectorAll<HTMLElement>("[data-reveal]")
    );
    for (const group of document.querySelectorAll<HTMLElement>("[data-reveal-group]")) {
      Array.from(group.children).forEach((child, index) => {
        const el = child as HTMLElement;
        el.style.setProperty("--reveal-delay", `${index * STAGGER_MS}ms`);
        targets.push(el);
      });
    }
    if (targets.length === 0) return;

    const io = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          entry.target.classList.add("reveal-in");
          io.unobserve(entry.target);
        }
      },
      // Fire once the element has cleared the bottom edge by a breath, so the
      // settle reads as arrival rather than as something flickering at the
      // fold.
      { rootMargin: "0px 0px -8% 0px" }
    );

    const viewportHeight = window.innerHeight;
    for (const el of targets) {
      const rect = el.getBoundingClientRect();
      if (rect.top < viewportHeight && rect.bottom > 0) continue;
      el.classList.add("reveal-armed");
      io.observe(el);
    }

    return () => io.disconnect();
  }, []);

  return null;
}
