"use client";

import { useEffect, useRef } from "react";

type ThreadsProps = {
  amplitude?: number;
  distance?: number;
  enableMouseInteraction?: boolean;
};

type Pointer = {
  x: number;
  y: number;
  active: boolean;
};

const BASE_LINE_COUNT = 22;
const SEGMENT_COUNT = 44;

export default function Threads({
  amplitude = 1.3,
  distance = 0,
  enableMouseInteraction = true
}: ThreadsProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const frameRef = useRef<number | null>(null);
  const pointerRef = useRef<Pointer>({ x: 0, y: 0, active: false });

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const context = canvas.getContext("2d");
    if (!context) return;

    const resize = () => {
      const parent = canvas.parentElement;
      if (!parent) return;

      const rect = parent.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      canvas.width = rect.width * dpr;
      canvas.height = rect.height * dpr;
      canvas.style.width = `${rect.width}px`;
      canvas.style.height = `${rect.height}px`;
      context.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    const updatePointer = (clientX: number, clientY: number) => {
      const rect = canvas.getBoundingClientRect();
      pointerRef.current = {
        x: clientX - rect.left,
        y: clientY - rect.top,
        active: true
      };
    };

    const handlePointerMove = (event: PointerEvent) => {
      if (!enableMouseInteraction) return;
      updatePointer(event.clientX, event.clientY);
    };

    const handlePointerLeave = () => {
      pointerRef.current.active = false;
    };

    const draw = (time: number) => {
      const rect = canvas.getBoundingClientRect();
      const width = rect.width;
      const height = rect.height;
      context.clearRect(0, 0, width, height);

      const lines = Math.max(14, Math.floor(BASE_LINE_COUNT + distance));
      const mouse = pointerRef.current;

      for (let lineIndex = 0; lineIndex < lines; lineIndex += 1) {
        const lineProgress = lineIndex / Math.max(1, lines - 1);
        const baseY = lineProgress * height;
        const brightness = 48 + Math.round(lineProgress * 120);
        const alpha = 0.14 + lineProgress * 0.18;

        context.beginPath();

        for (let segmentIndex = 0; segmentIndex <= SEGMENT_COUNT; segmentIndex += 1) {
          const segmentProgress = segmentIndex / SEGMENT_COUNT;
          const x = segmentProgress * width;

          const waveA =
            Math.sin(segmentProgress * 9 + time * 0.0014 + lineIndex * 0.35) *
            18 *
            amplitude;
          const waveB =
            Math.cos(segmentProgress * 14 - time * 0.001 + lineIndex * 0.18) *
            10 *
            amplitude;

          let mouseLift = 0;
          if (enableMouseInteraction && mouse.active) {
            const dx = x - mouse.x;
            const dy = baseY - mouse.y;
            const influence = Math.max(0, 1 - Math.sqrt(dx * dx + dy * dy) / 220);
            mouseLift = influence * 34 * amplitude;
          }

          const y = baseY + waveA + waveB - mouseLift;

          if (segmentIndex === 0) {
            context.moveTo(x, y);
          } else {
            context.lineTo(x, y);
          }
        }

        context.strokeStyle = `rgba(${brightness}, ${brightness}, ${brightness}, ${alpha})`;
        context.lineWidth = lineIndex % 3 === 0 ? 1.4 : 1;
        context.stroke();
      }

      frameRef.current = requestAnimationFrame(draw);
    };

    resize();
    frameRef.current = requestAnimationFrame(draw);

    window.addEventListener("resize", resize);
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerleave", handlePointerLeave);

    return () => {
      if (frameRef.current) {
        cancelAnimationFrame(frameRef.current);
      }
      window.removeEventListener("resize", resize);
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerleave", handlePointerLeave);
    };
  }, [amplitude, distance, enableMouseInteraction]);

  return <canvas ref={canvasRef} className="threads-canvas" />;
}
