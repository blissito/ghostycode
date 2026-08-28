"use client";

/**
 * <TiltFigure> — the hero capture answers the pointer with a whisper of
 * perspective (≤1.2°), like the printed plate being picked up off the desk.
 *
 * Restraint notes:
 * - transform only; nothing here can trigger layout.
 * - The rAF loop is idle unless the pointer is over the plate and a settle
 *   is still converging — no scroll listeners, no per-frame work at rest.
 * - Fine pointers with hover only: touch devices never attach a listener.
 * - prefers-reduced-motion is checked at mount and again per frame, so a
 *   mid-session change flattens the plate instead of stranding it tilted.
 */

import { useEffect, useRef, type ReactNode } from "react";

const MAX_DEG = 1.2;
const PERSPECTIVE_PX = 1200;
const EASE = 0.14;
const EPSILON = 0.002;

export function TiltFigure({
  className,
  children,
}: {
  className?: string;
  children: ReactNode;
}) {
  const ref = useRef<HTMLElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const reduceMq = window.matchMedia("(prefers-reduced-motion: reduce)");
    const fineMq = window.matchMedia("(hover: hover) and (pointer: fine)");
    if (reduceMq.matches || !fineMq.matches) return;

    let raf = 0;
    let targetX = 0;
    let targetY = 0;
    let currentX = 0;
    let currentY = 0;

    const settle = () => {
      raf = 0;
      if (reduceMq.matches) {
        el.style.transform = "";
        return;
      }
      currentX += (targetX - currentX) * EASE;
      currentY += (targetY - currentY) * EASE;
      if (Math.abs(targetX - currentX) < EPSILON && Math.abs(targetY - currentY) < EPSILON) {
        currentX = targetX;
        currentY = targetY;
      }
      el.style.transform = `perspective(${PERSPECTIVE_PX}px) rotateX(${(-currentY * MAX_DEG).toFixed(3)}deg) rotateY(${(currentX * MAX_DEG).toFixed(3)}deg)`;
      if (currentX !== targetX || currentY !== targetY) {
        raf = window.requestAnimationFrame(settle);
      }
    };
    const schedule = () => {
      if (!raf) raf = window.requestAnimationFrame(settle);
    };
    const onMove = (event: PointerEvent) => {
      if (reduceMq.matches) return;
      const rect = el.getBoundingClientRect();
      targetX = ((event.clientX - rect.left) / rect.width - 0.5) * 2;
      targetY = ((event.clientY - rect.top) / rect.height - 0.5) * 2;
      schedule();
    };
    const onLeave = () => {
      targetX = 0;
      targetY = 0;
      schedule();
    };

    el.addEventListener("pointermove", onMove);
    el.addEventListener("pointerleave", onLeave);
    return () => {
      el.removeEventListener("pointermove", onMove);
      el.removeEventListener("pointerleave", onLeave);
      if (raf) window.cancelAnimationFrame(raf);
    };
  }, []);

  return (
    <figure ref={ref} className={className}>
      {children}
    </figure>
  );
}
