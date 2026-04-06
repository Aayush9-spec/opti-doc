import Link from "next/link";

export default function LoginPage() {
  return (
    <div className="auth-layout">
      <section className="panel auth-panel auth-copy-panel">
        <p className="eyebrow">Authentication</p>
        <h1 className="section-title auth-title">
          Log back into the OptiDock control surface.
        </h1>
        <p className="lede">
          Resume live agent sessions, review container decisions, and keep your
          Docker workflows tied to a single operator identity.
        </p>

        <div className="auth-meta-list">
          <div className="auth-meta-item">
            <span className="signal-index">01</span>
            <p>Access terminal-native optimization workflows from one place.</p>
          </div>
          <div className="auth-meta-item">
            <span className="signal-index">02</span>
            <p>Keep environment, provider, and rollout context grouped per account.</p>
          </div>
          <div className="auth-meta-item">
            <span className="signal-index">03</span>
            <p>Move between local CLI work and dashboard-level visibility cleanly.</p>
          </div>
        </div>
      </section>

      <section className="panel auth-panel">
        <div className="section-heading">
          <p className="eyebrow">Login</p>
          <h2>Enter your operator credentials</h2>
        </div>

        <form className="auth-form">
          <label className="auth-field">
            <span>Email</span>
            <input type="email" placeholder="operator@optidock.dev" />
          </label>

          <label className="auth-field">
            <span>Password</span>
            <input type="password" placeholder="Enter your password" />
          </label>

          <div className="auth-form-row">
            <label className="auth-checkbox">
              <input type="checkbox" />
              <span>Keep this workstation trusted</span>
            </label>
            <Link href="/signup" className="auth-inline-link">
              Need an account?
            </Link>
          </div>

          <button type="submit" className="button button-primary auth-submit">
            Sign In
          </button>
        </form>
      </section>
    </div>
  );
}
