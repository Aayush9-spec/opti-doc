import Link from "next/link";

const signals = [
  "Deterministic Dockerfile analysis before any AI call",
  "Provider-flexible optimization architecture",
  "Terminal UX designed to feel like a real operations tool",
  "Benchmark-backed rollout decisions instead of blind rewrites"
];

const commandPreview = `optidock init
optidock doctor
optidock analyze ./my-app
optidock pipeline ./my-app
optidock providers`;

export default function HomePage() {
  return (
    <div className="page-stack">
      <section className="hero-grid">
        <div className="hero-copy">
          <p className="eyebrow">Rust-first autonomous Docker agent</p>
          <h1>Ship smaller, safer containers with a terminal that feels built for operators.</h1>
          <p className="lede">
            OptiDock AI analyzes Docker projects, surfaces optimization risks, and
            guides teams toward safer build, benchmark, and deployment decisions
            without turning the CLI into a toy.
          </p>

          <div className="button-row">
            <Link href="/docs" className="button button-primary">
              Read Docs
            </Link>
            <Link href="/use-cases" className="button button-secondary">
              Explore Use Cases
            </Link>
          </div>

          <div className="pill-row">
            <span className="mono-pill">Terminal-first</span>
            <span className="mono-pill">Monochrome landing system</span>
            <span className="mono-pill">AI-native container ops</span>
          </div>
        </div>

        <aside className="panel terminal-panel">
          <div className="panel-topline">
            <span>Live CLI direction</span>
            <span>v0.1</span>
          </div>
          <pre className="terminal-code">
            <code>{commandPreview}</code>
          </pre>
          <div className="terminal-footer">
            <span>doctor</span>
            <span>analyze</span>
            <span>pipeline</span>
          </div>
        </aside>
      </section>

      <section className="panel highlight-panel">
        <div className="section-heading">
          <p className="eyebrow">Why it exists</p>
          <h2>OptiDock focuses on the hard edge between developer ergonomics and production reality.</h2>
        </div>
        <div className="signal-grid">
          {signals.map((signal) => (
            <article key={signal} className="signal-card">
              <span className="signal-index">::</span>
              <p>{signal}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="content-grid">
        <article className="panel">
          <div className="section-heading">
            <p className="eyebrow">Install</p>
            <h2>Local-first setup</h2>
          </div>
          <p className="body-copy">
            Start with the included installer, then wire your preferred provider in
            `.optidock.env` or your shell environment.
          </p>
          <pre className="code-block">
            <code>{`chmod +x scripts/install.sh
./scripts/install.sh

optidock doctor
optidock analyze .`}</code>
          </pre>
        </article>

        <article className="panel">
          <div className="section-heading">
            <p className="eyebrow">Core Loop</p>
            <h2>Analyze, reason, validate, roll forward carefully.</h2>
          </div>
          <ol className="step-list">
            <li>Inspect Dockerfile and repository context.</li>
            <li>Generate findings with deterministic rules.</li>
            <li>Prepare provider-agnostic optimization input.</li>
            <li>Moderate rollout strategy before promotion.</li>
          </ol>
        </article>
      </section>
    </div>
  );
}
