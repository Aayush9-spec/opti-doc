const useCases = [
  {
    label: "Startup teams",
    title: "Shrink image size before every release starts hurting.",
    description:
      "Use OptiDock to catch broad COPY patterns, missing WORKDIR declarations, and runtime-user issues before they become slow CI builds and noisy production images."
  },
  {
    label: "Platform engineers",
    title: "Create a safer rollout conversation around container changes.",
    description:
      "Pipeline moderation helps teams pressure-test deployment targets, service roles, and rollout strategies before they promote optimized images."
  },
  {
    label: "Consultants",
    title: "Audit client repositories with a CLI that reads like an operator report.",
    description:
      "The terminal interface gives you a compact, presentation-friendly way to show issues, recommendations, and next actions without shipping a separate dashboard."
  },
  {
    label: "AI-native internal tools",
    title: "Keep deterministic checks in front of model output.",
    description:
      "OptiDock is useful when teams want AI leverage but still need explainable rule-based findings and a stronger safety posture around build and deploy decisions."
  }
];

export default function UseCasesPage() {
  return (
    <div className="page-stack">
      <section className="panel hero-panel">
        <p className="eyebrow">Use Cases</p>
        <h1 className="section-title">Where OptiDock is most effective right now.</h1>
        <p className="lede">
          This project works best when teams want automation around Docker quality
          and deployment reasoning, but still care about explainability, trust, and
          terminal speed.
        </p>
      </section>

      <section className="use-case-grid">
        {useCases.map((item) => (
          <article className="panel use-case-card" key={item.title}>
            <p className="eyebrow">{item.label}</p>
            <h2>{item.title}</h2>
            <p className="body-copy">{item.description}</p>
          </article>
        ))}
      </section>

      <section className="panel">
        <div className="section-heading">
          <p className="eyebrow">Practical Fit</p>
          <h2>Best for terminal-native workflows.</h2>
        </div>
        <p className="body-copy">
          OptiDock is intentionally strongest in repositories where the terminal is
          the primary control surface. If your team wants a credible CLI before a
          full dashboard, this is the right shape.
        </p>
      </section>
    </div>
  );
}
