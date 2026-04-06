import type { Metadata } from "next";
import Link from "next/link";
import "./globals.css";
import LetterGlitch from "@/components/LetterGlitch";

export const metadata: Metadata = {
  title: "OptiDock AI",
  description:
    "A monochrome Next.js landing site for OptiDock AI, the Rust-first autonomous Docker optimization agent."
};

const navItems = [
  { href: "/", label: "Overview" },
  { href: "/docs", label: "Documentation" },
  { href: "/use-cases", label: "Use Cases" }
];

export default function RootLayout({
  children
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>
        <div className="site-shell">
          <div className="site-background" aria-hidden="true">
            <LetterGlitch
              glitchColors={["#1a1a1a", "#4d4d4d", "#d8d8d8"]}
              glitchSpeed={55}
              centerVignette={false}
              outerVignette
              smooth
              characters="ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890<>[]{}()/\|"
            />
          </div>

          <div className="noise-layer" aria-hidden="true" />

          <header className="site-header">
            <Link href="/" className="brand-mark">
              <span className="brand-box">OD</span>
              <span className="brand-copy">
                <strong>OptiDock AI</strong>
                <small>Terminal-first container operations</small>
              </span>
            </Link>

            <nav className="site-nav">
              {navItems.map((item) => (
                <Link key={item.href} href={item.href} className="nav-link">
                  {item.label}
                </Link>
              ))}
            </nav>
          </header>

          <main className="page-frame">{children}</main>
        </div>
      </body>
    </html>
  );
}
