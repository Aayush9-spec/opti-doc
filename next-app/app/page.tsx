import LiquidEther from "@/components/liquid-ether"

export default function Page() {
  return (
    <main className="min-h-screen bg-[radial-gradient(circle_at_top,rgba(130,103,255,0.22),transparent_36%),linear-gradient(180deg,#050816_0%,#090d1f_45%,#05070f_100%)] text-white">
      <section className="relative overflow-hidden">
        <div className="absolute inset-0 bg-[linear-gradient(120deg,rgba(82,39,255,0.09),transparent_32%,rgba(255,159,252,0.08)_68%,transparent)]" />
        <div className="mx-auto grid min-h-screen max-w-7xl gap-14 px-6 py-10 lg:grid-cols-[1.05fr_0.95fr] lg:px-10 lg:py-14">
          <div className="relative z-10 flex flex-col justify-between">
            <div className="space-y-8">
              <div className="inline-flex w-fit items-center gap-3 rounded-full border border-white/15 bg-white/8 px-4 py-2 text-[11px] font-medium uppercase tracking-[0.32em] text-white/72 backdrop-blur-md">
                <span className="h-2 w-2 rounded-full bg-cyan-300 shadow-[0_0_18px_rgba(125,211,252,0.9)]" />
                Autonomous Container Operations
              </div>

              <div className="space-y-6">
                <p className="max-w-xl text-sm font-medium uppercase tracking-[0.26em] text-cyan-200/70">
                  OptiDock AI
                </p>
                <h1 className="max-w-4xl text-5xl font-semibold leading-[0.95] tracking-[-0.05em] text-white sm:text-6xl lg:text-7xl">
                  Self-optimizing Docker pipelines with a terminal-first control
                  plane.
                </h1>
                <p className="max-w-2xl text-base leading-8 text-white/70 sm:text-lg">
                  Analyze Dockerfiles, benchmark container variants, moderate
                  CI/CD rollouts, and choose safer deployments with a product
                  designed to feel like modern infrastructure software, not a
                  toy dashboard.
                </p>
              </div>

              <div className="flex flex-wrap gap-4">
                <a
                  href="#command-center"
                  className="inline-flex items-center justify-center rounded-full bg-white px-6 py-3 text-sm font-semibold text-slate-950 transition hover:scale-[1.02]"
                >
                  Explore Interface
                </a>
                <a
                  href="#agent-loop"
                  className="inline-flex items-center justify-center rounded-full border border-white/20 bg-white/5 px-6 py-3 text-sm font-semibold text-white backdrop-blur-md transition hover:bg-white/10"
                >
                  See Agent Loop
                </a>
              </div>
            </div>

            <div className="mt-12 grid gap-4 sm:grid-cols-3">
              {[
                ["41%", "Image size reduction"],
                ["3.2x", "Faster validation loop"],
                ["0-regret", "Rollback-first releases"],
              ].map(([value, label]) => (
                <div
                  key={label}
                  className="rounded-[28px] border border-white/10 bg-white/6 p-5 backdrop-blur-md"
                >
                  <div className="text-3xl font-semibold tracking-[-0.04em] text-white">
                    {value}
                  </div>
                  <div className="mt-2 text-sm leading-6 text-white/58">
                    {label}
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div
            id="command-center"
            className="relative z-10 flex items-center justify-center"
          >
            <div className="w-full max-w-3xl rounded-[32px] border border-white/12 bg-[#090d1f]/70 p-3 shadow-[0_30px_120px_rgba(13,18,38,0.6)] backdrop-blur-xl">
              <div className="rounded-[26px] border border-white/8 bg-[#060915]/92 p-4 sm:p-5">
                <div className="mb-4 flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="h-3 w-3 rounded-full bg-[#ff6a88]" />
                    <span className="h-3 w-3 rounded-full bg-[#ffd166]" />
                    <span className="h-3 w-3 rounded-full bg-[#5ef2a3]" />
                  </div>
                  <div className="rounded-full border border-cyan-300/20 bg-cyan-300/8 px-3 py-1 text-[11px] uppercase tracking-[0.22em] text-cyan-200/72">
                    Live Command Center
                  </div>
                </div>

                <div className="relative overflow-hidden rounded-[24px] border border-white/8 bg-[#04070f]">
                  <div style={{ width: "100%", height: 600, position: "relative" }}>
                    <LiquidEther
                      colors={["#5227FF", "#FF9FFC", "#B19EEF"]}
                      mouseForce={20}
                      cursorSize={100}
                      isViscous
                      viscous={30}
                      iterationsViscous={32}
                      iterationsPoisson={32}
                      resolution={0.5}
                      isBounce={false}
                      autoDemo
                      autoSpeed={0.5}
                      autoIntensity={2.2}
                      takeoverDuration={0.25}
                      autoResumeDelay={3000}
                      autoRampDuration={0.6}
                      color0="#5227FF"
                      color1="#FF9FFC"
                      color2="#B19EEF"
                    />
                  </div>

                  <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(180deg,rgba(4,7,15,0.16),rgba(4,7,15,0.48))]" />

                  <div className="absolute inset-x-0 top-0 flex items-center justify-between border-b border-white/10 bg-black/22 px-5 py-4 font-mono text-xs text-white/70 backdrop-blur-md">
                    <span>optidock pipeline ./payments-service</span>
                    <span>staging • canary</span>
                  </div>

                  <div className="absolute inset-x-0 bottom-0 grid gap-3 p-5 sm:grid-cols-2">
                    <div className="rounded-2xl border border-cyan-300/18 bg-slate-950/72 p-4 backdrop-blur-md">
                      <div className="text-[11px] uppercase tracking-[0.24em] text-cyan-200/68">
                        Agent verdict
                      </div>
                      <div className="mt-2 text-xl font-semibold tracking-[-0.04em] text-white">
                        Deploy optimized image
                      </div>
                      <div className="mt-2 text-sm leading-6 text-white/62">
                        Reduced layers, preserved startup latency, and staged a
                        canary path with rollback guardrails.
                      </div>
                    </div>
                    <div className="rounded-2xl border border-white/10 bg-white/8 p-4 backdrop-blur-md">
                      <div className="grid grid-cols-3 gap-3 text-center">
                        {[
                          ["128MB", "Before"],
                          ["74MB", "After"],
                          ["PASS", "Health"],
                        ].map(([value, label]) => (
                          <div key={label}>
                            <div className="text-lg font-semibold text-white">
                              {value}
                            </div>
                            <div className="text-[11px] uppercase tracking-[0.2em] text-white/48">
                              {label}
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section
        id="agent-loop"
        className="mx-auto max-w-7xl px-6 pb-16 lg:px-10 lg:pb-24"
      >
        <div className="grid gap-6 lg:grid-cols-[0.9fr_1.1fr]">
          <div className="rounded-[32px] border border-white/10 bg-white/6 p-8 backdrop-blur-xl">
            <p className="text-sm uppercase tracking-[0.3em] text-cyan-200/65">
              Agent Loop
            </p>
            <h2 className="mt-4 text-3xl font-semibold tracking-[-0.04em] text-white">
              Built for real container decisions, not just pretty suggestions.
            </h2>
            <p className="mt-4 max-w-xl text-base leading-8 text-white/66">
              OptiDock inspects Docker context, rewrites smarter candidates,
              benchmarks before promotion, and keeps rollback decisions close to
              the deployment surface.
            </p>
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            {[
              {
                title: "Analyze",
                body: "Parse Dockerfiles, detect anti-patterns, and build deterministic findings before any model call.",
              },
              {
                title: "Optimize",
                body: "Generate provider-agnostic proposals through OpenAI, Claude, Gemini, OpenRouter, or local models.",
              },
              {
                title: "Benchmark",
                body: "Compare baseline and optimized containers with image-size, startup, and smoke-test feedback.",
              },
              {
                title: "Moderate",
                body: "Choose rollout strategies, flag weak pipeline assumptions, and bias the system toward safe promotion.",
              },
            ].map((card) => (
              <article
                key={card.title}
                className="group rounded-[28px] border border-white/10 bg-[linear-gradient(180deg,rgba(255,255,255,0.08),rgba(255,255,255,0.04))] p-6 transition hover:-translate-y-1 hover:border-cyan-300/30"
              >
                <div className="text-sm uppercase tracking-[0.28em] text-cyan-200/68">
                  {card.title}
                </div>
                <p className="mt-4 text-lg font-medium leading-8 text-white/86">
                  {card.body}
                </p>
              </article>
            ))}
          </div>
        </div>
      </section>
    </main>
  )
}
