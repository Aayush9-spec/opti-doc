"use client";

import { useEffect, useRef } from "react";

type LetterGlitchProps = {
  glitchColors?: string[];
  glitchSpeed?: number;
  centerVignette?: boolean;
  outerVignette?: boolean;
  smooth?: boolean;
  characters?: string;
};

type LetterState = {
  char: string;
  color: string;
  sourceColor: string;
  targetColor: string;
  colorProgress: number;
};

const fontSize = 16;
const charWidth = 10;
const charHeight = 20;

export default function LetterGlitch({
  glitchColors = ["#1a1a1a", "#4d4d4d", "#d8d8d8"],
  glitchSpeed = 50,
  centerVignette = false,
  outerVignette = true,
  smooth = true,
  characters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ!@#$&*()-_+=/[]{};:<>.,0123456789"
}: LetterGlitchProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const animationRef = useRef<number | null>(null);
  const letters = useRef<LetterState[]>([]);
  const grid = useRef({ columns: 0, rows: 0 });
  const context = useRef<CanvasRenderingContext2D | null>(null);
  const lastGlitchTime = useRef(Date.now());
  const lettersAndSymbols = useRef(Array.from(characters));

  useEffect(() => {
    lettersAndSymbols.current = Array.from(characters);
  }, [characters]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    context.current = canvas.getContext("2d");
    if (!context.current) return;

    const getRandomChar = () =>
      lettersAndSymbols.current[
        Math.floor(Math.random() * lettersAndSymbols.current.length)
      ];

    const getRandomColor = () =>
      glitchColors[Math.floor(Math.random() * glitchColors.length)];

    const hexToRgb = (hex: string) => {
      const shorthandRegex = /^#?([a-f\d])([a-f\d])([a-f\d])$/i;
      const normalized = hex.replace(shorthandRegex, (_m, r, g, b) => {
        return r + r + g + g + b + b;
      });

      const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(normalized);
      return result
        ? {
            r: parseInt(result[1], 16),
            g: parseInt(result[2], 16),
            b: parseInt(result[3], 16)
          }
        : null;
    };

    const interpolateColor = (
      start: { r: number; g: number; b: number },
      end: { r: number; g: number; b: number },
      factor: number
    ) => {
      const result = {
        r: Math.round(start.r + (end.r - start.r) * factor),
        g: Math.round(start.g + (end.g - start.g) * factor),
        b: Math.round(start.b + (end.b - start.b) * factor)
      };
      return `rgb(${result.r}, ${result.g}, ${result.b})`;
    };

    const calculateGrid = (width: number, height: number) => {
      const columns = Math.ceil(width / charWidth);
      const rows = Math.ceil(height / charHeight);
      return { columns, rows };
    };

    const initializeLetters = (columns: number, rows: number) => {
      grid.current = { columns, rows };
      const totalLetters = columns * rows;
      letters.current = Array.from({ length: totalLetters }, () => {
        const color = getRandomColor();
        return {
          char: getRandomChar(),
          color,
          sourceColor: color,
          targetColor: getRandomColor(),
          colorProgress: 1
        };
      });
    };

    const drawVignette = (ctx: CanvasRenderingContext2D, width: number, height: number) => {
      if (outerVignette) {
        const outer = ctx.createRadialGradient(
          width / 2,
          height / 2,
          width * 0.1,
          width / 2,
          height / 2,
          width * 0.78
        );
        outer.addColorStop(0, "rgba(0, 0, 0, 0)");
        outer.addColorStop(1, "rgba(0, 0, 0, 0.96)");
        ctx.fillStyle = outer;
        ctx.fillRect(0, 0, width, height);
      }

      if (centerVignette) {
        const inner = ctx.createRadialGradient(
          width / 2,
          height / 2,
          width * 0.02,
          width / 2,
          height / 2,
          width * 0.45
        );
        inner.addColorStop(0, "rgba(0, 0, 0, 0.7)");
        inner.addColorStop(1, "rgba(0, 0, 0, 0)");
        ctx.fillStyle = inner;
        ctx.fillRect(0, 0, width, height);
      }
    };

    const drawLetters = () => {
      if (!context.current) return;
      const ctx = context.current;
      const { width, height } = canvas.getBoundingClientRect();
      ctx.clearRect(0, 0, width, height);
      ctx.font = `${fontSize}px ui-monospace, SFMono-Regular, Menlo, monospace`;
      ctx.textBaseline = "top";

      letters.current.forEach((letter, index) => {
        const x = (index % grid.current.columns) * charWidth;
        const y = Math.floor(index / grid.current.columns) * charHeight;
        ctx.fillStyle = letter.color;
        ctx.fillText(letter.char, x, y);
      });

      drawVignette(ctx, width, height);
    };

    const resizeCanvas = () => {
      const parent = canvas.parentElement;
      if (!parent || !context.current) return;

      const dpr = window.devicePixelRatio || 1;
      const rect = parent.getBoundingClientRect();
      canvas.width = rect.width * dpr;
      canvas.height = rect.height * dpr;
      canvas.style.width = `${rect.width}px`;
      canvas.style.height = `${rect.height}px`;
      context.current.setTransform(dpr, 0, 0, dpr, 0, 0);

      const { columns, rows } = calculateGrid(rect.width, rect.height);
      initializeLetters(columns, rows);
      drawLetters();
    };

    const updateLetters = () => {
      const updateCount = Math.max(1, Math.floor(letters.current.length * 0.05));

      for (let index = 0; index < updateCount; index += 1) {
        const targetIndex = Math.floor(Math.random() * letters.current.length);
        const targetLetter = letters.current[targetIndex];
        if (!targetLetter) continue;

        targetLetter.char = getRandomChar();
        targetLetter.sourceColor = targetLetter.color;
        targetLetter.targetColor = getRandomColor();

        if (!smooth) {
          targetLetter.color = targetLetter.targetColor;
          targetLetter.colorProgress = 1;
        } else {
          targetLetter.colorProgress = 0;
        }
      }
    };

    const handleSmoothTransitions = () => {
      let needsRedraw = false;

      letters.current.forEach((letter) => {
        if (letter.colorProgress >= 1) return;

        letter.colorProgress += 0.05;
        if (letter.colorProgress > 1) {
          letter.colorProgress = 1;
        }

        const startRgb = hexToRgb(letter.sourceColor);
        const endRgb = hexToRgb(letter.targetColor);
        if (!startRgb || !endRgb) return;

        letter.color = interpolateColor(startRgb, endRgb, letter.colorProgress);
        needsRedraw = true;
      });

      if (needsRedraw) {
        drawLetters();
      }
    };

    const animate = () => {
      const now = Date.now();
      if (now - lastGlitchTime.current >= glitchSpeed) {
        updateLetters();
        drawLetters();
        lastGlitchTime.current = now;
      }

      if (smooth) {
        handleSmoothTransitions();
      }

      animationRef.current = requestAnimationFrame(animate);
    };

    resizeCanvas();
    animate();

    let resizeTimeout: ReturnType<typeof setTimeout>;
    const handleResize = () => {
      clearTimeout(resizeTimeout);
      resizeTimeout = setTimeout(() => {
        if (animationRef.current) {
          cancelAnimationFrame(animationRef.current);
        }
        resizeCanvas();
        animate();
      }, 100);
    };

    window.addEventListener("resize", handleResize);

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
      window.removeEventListener("resize", handleResize);
    };
  }, [centerVignette, glitchColors, glitchSpeed, outerVignette, smooth]);

  return <canvas ref={canvasRef} className="letter-glitch-canvas" />;
}
