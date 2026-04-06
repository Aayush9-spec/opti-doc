import Threads from "@/components/Threads";

const docsSections = [
  {
    title: "Installation",
    body:
      "Install the CLI from the repository root, then validate the machine with `optidock doctor`. The current workflow expects Rust, Cargo, and Docker to be available locally.",
    code: `chmod +x scripts/install.sh
./scripts/install.sh
optidock doctor`
  },
  {
    title: "Analysis",
    body:
      "Run deterministic Dockerfile analysis before asking any model to rewrite anything. This gives you a baseline set of findings and keeps the tool useful even without provider credentials.",
    code: `optidock analyze ./my-app
optidock analyze . --json`
  },
  {
    title: "Pipeline Moderation",
    body:
      "Use the pipeline mode to model deployment safety and rollout strategy. OptiDock surfaces missing image sources, uncertain targets, and weak scaling assumptions before promotion.",
    code: `optidock pipeline ./my-app
optidock pipeline . --json`
  }
];

export default function DocsPage() {
  return (
    <div className="page-stack">
      <section className="panel hero-panel docs-hero">
        <div className="docs-threads" aria-hidden="true">
          <Threads amplitude={1.3} distance={0} enableMouseInteraction />
        </div>
        <div className="docs-hero-copy">
          <p className="eyebrow">Documentation</p>
          <h1 className="section-title">Run OptiDock like an actual toolchain, not a demo script.</h1>
          <p className="lede">
            These docs are structured around the current project state: terminal-first,
            Rust-powered, and intentionally useful before deeper AI automation lands.
          </p>
        </div>
      </section>

      <section className="docs-grid">
        {docsSections.map((section) => (
          <article className="panel" key={section.title}>
            <div className="section-heading">
              <p className="eyebrow">Guide</p>
              <h2>{section.title}</h2>
            </div>
            <p className="body-copy">{section.body}</p>
            <pre className="code-block">
              <code>{section.code}</code>
            </pre>
          </article>
        ))}
      </section>

      <section className="panel">
        <div className="section-heading">
          <p className="eyebrow">Provider Strategy</p>
          <h2>Keep the optimization layer portable.</h2>
        </div>
        <p className="body-copy">
          The current runtime is designed around provider flexibility: OpenAI,
          Anthropic, Gemini, OpenRouter, Ollama, and local OpenAI-compatible
          endpoints. The goal is to keep orchestration logic stable even when the
          backend model changes.
        </p>
      </section>
    </div>
  );
}
