import Link from "next/link";

export default function SignupPage() {
  return (
    <div className="auth-layout">
      <section className="panel auth-panel auth-copy-panel">
        <p className="eyebrow">New Workspace</p>
        <h1 className="section-title auth-title">
          Create an OptiDock account for live container operations.
        </h1>
        <p className="lede">
          Set up your workspace, connect preferred providers, and onboard teams
          into a terminal-first optimization flow designed for production-minded
          engineering.
        </p>

        <div className="auth-meta-list">
          <div className="auth-meta-item">
            <span className="signal-index">A1</span>
            <p>Configure hosted or local AI providers behind one operator profile.</p>
          </div>
          <div className="auth-meta-item">
            <span className="signal-index">A2</span>
            <p>Prepare audit-friendly optimization and rollout workflows per team.</p>
          </div>
          <div className="auth-meta-item">
            <span className="signal-index">A3</span>
            <p>Bring the CLI, docs, and future live dashboard under one identity layer.</p>
          </div>
        </div>
      </section>

      <section className="panel auth-panel">
        <div className="section-heading">
          <p className="eyebrow">Sign Up</p>
          <h2>Provision a new operator account</h2>
        </div>

        <form className="auth-form">
          <label className="auth-field">
            <span>Full name</span>
            <input type="text" placeholder="Aayush Singh" />
          </label>

          <label className="auth-field">
            <span>Work email</span>
            <input type="email" placeholder="team@optidock.dev" />
          </label>

          <label className="auth-field">
            <span>Password</span>
            <input type="password" placeholder="Create a secure password" />
          </label>

          <label className="auth-field">
            <span>Workspace name</span>
            <input type="text" placeholder="OptiDock Ops" />
          </label>

          <div className="auth-form-row">
            <label className="auth-checkbox">
              <input type="checkbox" />
              <span>I agree to the workspace security and provider access terms</span>
            </label>
          </div>

          <button type="submit" className="button button-primary auth-submit">
            Create Account
          </button>

          <p className="auth-switch-copy">
            Already onboarded?{" "}
            <Link href="/login" className="auth-inline-link">
              Log in here
            </Link>
          </p>
        </form>
      </section>
    </div>
  );
}
