"use client";

import { useRef, useEffect, useState, ReactNode } from "react";

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
  const [copies, setCopies] = useState(3);

  useEffect(() => {
    const track = trackRef.current;
    const container = containerRef.current;
    if (!track || !container) return;

    // Measure single copy width after render
    const measure = () => {
      const totalWidth = track.scrollWidth;
      const single = totalWidth / copies;
      singleWidthRef.current = single;

      // Need enough copies to fill 3x container for seamless wrap
      const containerWidth = container.offsetWidth;
      const needed = Math.max(3, Math.ceil((containerWidth * 3) / single));
      if (needed !== copies) {
        setCopies(needed);
        return false; // re-render needed
      }
      return true;
    };

    // Wait for render then measure
    const id = requestAnimationFrame(() => {
      if (!measure()) return; // will re-trigger via useEffect

      const tick = (time: number) => {
        if (prevTimeRef.current === 0) prevTimeRef.current = time;
        const dt = (time - prevTimeRef.current) / 1000;
        prevTimeRef.current = time;

        if (!pausedRef.current && singleWidthRef.current > 0) {
          offsetRef.current -= speed * dt;

          // When first copy has scrolled off, snap forward by one copy width
          if (Math.abs(offsetRef.current) >= singleWidthRef.current) {
            offsetRef.current += singleWidthRef.current;
          }

          track.style.transform = `translateX(${offsetRef.current}px)`;
        }

        animRef.current = requestAnimationFrame(tick);
      };

      animRef.current = requestAnimationFrame(tick);
    });

    return () => {
      cancelAnimationFrame(id);
      cancelAnimationFrame(animRef.current);
    };
  }, [copies, speed, children]);

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
        {Array.from({ length: copies }).map((_, i) => (
          <span key={i} className="contents">
            {children}
          </span>
        ))}
      </div>
    </div>
  );
}
