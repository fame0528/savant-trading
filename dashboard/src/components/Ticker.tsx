"use client";

import { useRef, useEffect, useLayoutEffect, ReactNode } from "react";

interface TickerProps {
  speed?: number;
  children: ReactNode;
}

export default function Ticker({ speed = 60, children }: TickerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const trackRef = useRef<HTMLDivElement>(null);
  const offsetRef = useRef(0);
  const pausedRef = useRef(false);
  const singleWidthRef = useRef(0);
  const animRef = useRef<number>(0);
  const prevTimeRef = useRef<number>(0);

  // Measure single copy width: track has 3 identical copies, so scrollWidth / 3
  useLayoutEffect(() => {
    const track = trackRef.current;
    if (!track) return;
    const w = track.scrollWidth / 3;
    if (w > 0) singleWidthRef.current = w;
  });

  // Animation loop — runs once, never re-starts
  useEffect(() => {
    const track = trackRef.current;
    if (!track) return;

    const tick = (time: number) => {
      if (prevTimeRef.current === 0) {
        prevTimeRef.current = time;
      }
      const dt = (time - prevTimeRef.current) / 1000;
      prevTimeRef.current = time;

      if (!pausedRef.current && singleWidthRef.current > 0) {
        offsetRef.current -= speed * dt;

        // Snap forward by one copy width when first copy scrolls off
        if (Math.abs(offsetRef.current) >= singleWidthRef.current) {
          offsetRef.current += singleWidthRef.current;
        }

        track.style.transform = `translateX(${offsetRef.current}px)`;
      }

      animRef.current = requestAnimationFrame(tick);
    };

    animRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(animRef.current);
  }, [speed]);

  return (
    <div
      ref={containerRef}
      className="shrink-0 overflow-hidden bg-[var(--panel)] border border-[var(--line)] backdrop-blur-md h-6 flex items-center"
      onMouseEnter={() => { pausedRef.current = true; }}
      onMouseLeave={() => { pausedRef.current = false; }}
    >
      <div
        ref={trackRef}
        className="flex items-center gap-8 whitespace-nowrap px-4 will-change-transform"
      >
        <span className="contents">{children}</span>
        <span className="contents">{children}</span>
        <span className="contents">{children}</span>
      </div>
    </div>
  );
}
