"use client"

import type { CSSProperties } from "react"

type LiquidEtherProps = {
  colors?: [string, string, string] | string[]
  mouseForce?: number
  cursorSize?: number
  isViscous?: boolean
  viscous?: number
  iterationsViscous?: number
  iterationsPoisson?: number
  resolution?: number
  isBounce?: boolean
  autoDemo?: boolean
  autoSpeed?: number
  autoIntensity?: number
  takeoverDuration?: number
  autoResumeDelay?: number
  autoRampDuration?: number
  color0?: string
  color1?: string
  color2?: string
}

export default function LiquidEther({
  colors = ["#5227FF", "#FF9FFC", "#B19EEF"],
  mouseForce = 20,
  cursorSize = 100,
  isViscous = true,
  viscous = 30,
  iterationsViscous = 32,
  iterationsPoisson = 32,
  resolution = 0.5,
  isBounce = false,
  autoDemo = true,
  autoSpeed = 0.5,
  autoIntensity = 2.2,
  takeoverDuration = 0.25,
  autoResumeDelay = 3000,
  autoRampDuration = 0.6,
  color0,
  color1,
  color2,
}: LiquidEtherProps) {
  const palette = [color0 ?? colors[0], color1 ?? colors[1], color2 ?? colors[2]]

  return (
    <div
      className="liquid-ether"
      aria-hidden="true"
      style={
        {
          "--le-color-0": palette[0],
          "--le-color-1": palette[1],
          "--le-color-2": palette[2],
          "--le-cursor-size": `${cursorSize}px`,
          "--le-mouse-force": String(mouseForce),
          "--le-viscous": String(isViscous ? viscous : 0),
          "--le-iterations-v": String(iterationsViscous),
          "--le-iterations-p": String(iterationsPoisson),
          "--le-resolution": String(resolution),
          "--le-bounce": String(Number(isBounce)),
          "--le-auto-demo": String(Number(autoDemo)),
          "--le-auto-speed": `${autoSpeed}s`,
          "--le-auto-intensity": String(autoIntensity),
          "--le-takeover": `${takeoverDuration}s`,
          "--le-resume-delay": `${autoResumeDelay}ms`,
          "--le-ramp": `${autoRampDuration}s`,
        } as CSSProperties
      }
    >
      <div className="liquid-ether__orb liquid-ether__orb--a" />
      <div className="liquid-ether__orb liquid-ether__orb--b" />
      <div className="liquid-ether__orb liquid-ether__orb--c" />
      <div className="liquid-ether__grid" />
      <div className="liquid-ether__glow" />
    </div>
  )
}
