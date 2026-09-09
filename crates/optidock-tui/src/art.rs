//! ASCII art library — Docker, container, recovery, and brand illustrations.

use std::collections::HashMap;

/// A piece of ASCII art with metadata.
#[derive(Debug, Clone)]
pub struct AsciiArt {
    pub lines: Vec<&'static str>,
    pub width: usize,
    pub height: usize,
    pub tags: Vec<&'static str>,
}

impl AsciiArt {
    /// Create new art from raw lines (computed width/height).
    pub fn new(lines: &'static [&'static str], tags: &'static [&'static str]) -> Self {
        let mut width = 0;
        let mut i = 0;
        while i < lines.len() {
            let len = lines[i].len();
            if len > width { width = len; }
            i += 1;
        }
        Self { lines: lines.to_vec(), width, height: lines.len(), tags: tags.to_vec() }
    }

    /// Render with optional color per line (for theme integration).
    pub fn render(&self, style_fn: impl Fn(usize, &str) -> String) -> String {
        self.lines.iter().enumerate().map(|(i, line)| style_fn(i, line)).collect::<Vec<_>>().join("\n")
    }

    /// Render plain (no color).
    pub fn plain(&self) -> String {
        self.lines.join("\n")
    }

    /// Center within a given width.
    pub fn centered(&self, width: usize) -> String {
        let pad = width.saturating_sub(self.width) / 2;
        let prefix = " ".repeat(pad);
        self.lines.iter().map(|line| format!("{}{}", prefix, line)).collect::<Vec<_>>().join("\n")
    }
}

/// Central art library — register and retrieve by key.
#[derive(Debug, Default)]
pub struct ArtLibrary {
    art: HashMap<&'static str, AsciiArt>,
}

impl ArtLibrary {
    /// Create with all built-in art pre-registered.
    pub fn new() -> Self {
        let mut lib = Self { art: HashMap::new() };
        lib.register_builtins();
        lib
    }

    fn register_builtins(&mut self) {
        macro_rules! add {
            ($key:expr, $lines:expr, $($tag:expr),*) => {
                self.art.insert($key, AsciiArt::new($lines, &[$($tag),*]));
            };
        }

        // ─── BRAND ─────────────────────────────────────────────────────────────
        add!("brand.logo", &[
            "    ██████╗ ██████╗ ███████╗██████╗ ██╗    ██╗",
            "   ██╔═══██╗██╔══██╗██╔════╝██╔══██╗██║    ██║",
            "   ██║   ██║██████╔╝█████╗  ██████╔╝██║ █╗ ██║",
            "   ██║   ██║██╔══██╗██╔══╝  ██╔══██╗██║███╗██║",
            "   ╚██████╔╝██║  ██║███████╗██║  ██║╚███╔███╔╝",
            "    ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝ ╚══╝╚══╝ ",
        ], "brand", "logo", "banner");

        add!("brand.logo_small", &[
            "  ██████╗ ██████╗ ███████╗██████╗ ",
            "  ██╔══██╗██╔══██╗██╔════╝██╔══██╗",
            "  ██████╔╝██████╔╝█████╗  ██████╔╝",
            "  ██╔═══╝ ██╔══██╗██╔══╝  ██╔══██╗",
            "  ██║     ██║  ██║███████╗██║  ██║",
            "  ╚═╝     ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝",
        ], "brand", "logo", "small");

        add!("brand.wordmark", &[
            "  ██████╗ ██████╗ ███████╗██████╗ ██╗    ██╗",
            "  ██╔══██╗██╔══██╗██╔════╝██╔══██╗██║    ██║",
            "  ██████╔╝██████╔╝█████╗  ██████╔╝██║ █╗ ██║",
            "  ██╔═══╝ ██╔══██╗██╔══╝  ██╔══██╗██║███╗██║",
            "  ██║     ██║  ██║███████╗██║  ██║╚███╔███╔╝",
            "  ╚═╝     ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝ ╚══╝╚══╝ ",
        ], "brand", "wordmark");

        add!("brand.mini", &[
            "╔══════════════════════════════╗",
            "║  O P T I D O C K  A I      ║",
            "║  Autonomous Docker Agent   ║",
            "╚══════════════════════════════╝",
        ], "brand", "mini", "boxed");

        // ─── DOCKER / CONTAINERS ───────────────────────────────────────────────
        add!("docker.whale", &[
            "                    ##        ",
            "             ## ## ##       ",
            "          ## ## ## ## ##    ",
            "        ## ## ## ## ##      ",
            "      ## ## ## ## ##        ",
            "    ## ## ## ## ##          ",
            "   ## ## ## ##              ",
            "  ## ## ##                  ",
            " ## ##                      ",
            "##                          ",
        ], "docker", "whale", "mascot");

        add!("docker.whale_small", &[
            "      ##        ",
            "   ## ## ##     ",
            "  ## ## ## ##   ",
            " ## ## ## ##    ",
            "## ## ## ##     ",
            " ## ## ##       ",
            "  ## ##         ",
            "   ##           ",
        ], "docker", "whale", "small");

        add!("docker.container", &[
            "┌─────────────────────────┐",
            "│ ████████████████████████ │",
            "│ ██ CONTAINER RUNNING  ██ │",
            "│ ████████████████████████ │",
            "│ ██  [cpu: 12%] [mem:45%] ██ │",
            "│ ████████████████████████ │",
            "└─────────────────────────┘",
        ], "docker", "container", "box");

        add!("docker.container_stack", &[
            "    ┌─────────────┐",
            "    │  CONTAINER  │",
            "    │   (app)     │",
            "    └──────┬──────┘",
            "           │",
            "    ┌──────┴──────┐",
            "    │  CONTAINER  │",
            "    │  (worker)   │",
            "    └──────┬──────┘",
            "           │",
            "    ┌──────┴──────┐",
            "    │  CONTAINER  │",
            "    │   (db)      │",
            "    └─────────────┘",
        ], "docker", "stack", "composition");

        add!("docker.layers", &[
            "  ┌─────────────────────┐  ← App layer",
            "  │   YOUR CODE         │",
            "  ├─────────────────────┤  ← Runtime layer",
            "  │   node:20-alpine    │",
            "  ├─────────────────────┤  ← Base layer",
            "  │   alpine:3.19       │",
            "  ├─────────────────────┤  ← Scratch",
            "  │                     │",
            "  └─────────────────────┘",
        ], "docker", "layers", "image");

        add!("docker.compose", &[
            "  services:",
            "  ┌─ app ─────────────────┐",
            "  │ image: myapp:latest   │",
            "  │ ports: [8080:8080]    │",
            "  │ depends_on: [db, redis]│",
            "  └───────────────────────┘",
            "  ┌─ db ──────────────────┐",
            "  │ image: postgres:16    │",
            "  │ volumes: [pgdata]     │",
            "  └───────────────────────┘",
            "  ┌─ redis ───────────────┐",
            "  │ image: redis:7-alpine │",
            "  └───────────────────────┘",
        ], "docker", "compose", "stack");

        // ─── RECOVERY / AGENT ──────────────────────────────────────────────────
        add!("recovery.brain", &[
            "    ╔═════════════════════╗",
            "    ║     O P T I B R A I N    ║",
            "    ╠═════════════════════╣",
            "    ║  ████████████████████  ║",
            "    ║  ██  NEURAL NET   ██  ║",
            "    ║  ████████████████████  ║",
            "    ║  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  ║",
            "    ║  ▒▒  REASONING   ▒▒  ║",
            "    ╚═════════════════════╝",
        ], "recovery", "brain", "ai");

        add!("recovery.pipeline", &[
            "  ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐",
            "  │ OBSERVE │──▶│ ANALYZE │──▶│  PLAN   │──▶│ EXECUTE │",
            "  └────┬────┘   └────┬────┘   └────┬────┘   └────┬────┘",
            "       │             │             │             │",
            "       ▼             ▼             ▼             ▼",
            "   ┌────────┐   ┌──────────┐  ┌──────────┐  ┌──────────┐",
            "   │ Logs   │   │Root Cause│  │ Actions  │  │ Verify   │",
            "   │ Metrics│   │Confidence│  │ Priority │  │ Health   │",
            "   └────────┘   └──────────┘  └──────────┘  └──────────┘",
        ], "recovery", "pipeline", "flow");

        add!("recovery.escalation", &[
            "  ╔══════════════════════════════════════╗",
            "  ║       ⚠  ESCALATION TRIGGERED  ⚠       ║",
            "  ╠══════════════════════════════════════╣",
            "  ║  Deterministic recovery exhausted      ║",
            "  ║  OptiBrain advisory diagnosis active   ║",
            "  ║                                        ║",
            "  ║  ┌──────────────────────────────────┐  ║",
            "  ║  │ Refined cause: ResourceExhaustion│  ║",
            "  ║  │ Confidence: 99%                  │  ║",
            "  ║  │ Action:    RestartContainer      │  ║",
            "  ║  └──────────────────────────────────┘  ║",
            "  ╚══════════════════════════════════════╝",
        ], "recovery", "escalation", "alert");

        add!("recovery.agent", &[
            "  ┌─────────────────────────────────────┐",
            "  │         LOCAL AGENT #abc123          │",
            "  ├─────────────────────────────────────┤",
            "  │  Status:    ● HEALTHY               │",
            "  │  Container: myapp (running)         │",
            "  │  CPU:       ████████░░  12%         │",
            "  │  Memory:    ████████████░░  45%     │",
            "  │  Uptime:    2h 34m                  │",
            "  └─────────────────────────────────────┘",
        ], "recovery", "agent", "status");

        add!("recovery.master", &[
            "  ╔═══════════════════════════════════════╗",
            "  ║        MASTER AGENT SUPERVISOR         ║",
            "  ╠═══════════════════════════════════════╣",
            "  ║  Infrastructure:  HEALTHY              ║",
            "  ║  Containers:      12 monitored         ║",
            "  ║  Escalations:     0 active             ║",
            "  ║  Recovery rate:   94%                  ║",
            "  ╚═══════════════════════════════════════╝",
        ], "recovery", "master", "dashboard");

        // ─── STATUS INDICATORS ─────────────────────────────────────────────────
        add!("status.healthy", &[
            "  ● HEALTHY",
        ], "status", "healthy", "inline");

        add!("status.warning", &[
            "  ⚠ WARNING",
        ], "status", "warning", "inline");

        add!("status.critical", &[
            "  ✗ CRITICAL",
        ], "status", "critical", "inline");

        add!("status.escalated", &[
            "  ▲ ESCALATED",
        ], "status", "escalated", "inline");

        add!("status.running", &[
            "  ◓ RUNNING",
        ], "status", "running", "inline");

        // ─── PROGRESS / LOADING ────────────────────────────────────────────────
        add!("progress.bar", &[
            "  ┌─────────────────────────────────────┐",
            "  │ ████████████████████░░░░░░░░░░░░ 60% │",
            "  └─────────────────────────────────────┘",
        ], "progress", "bar");

        add!("progress.steps", &[
            "  ▸ Step 1: Collect observation        ✓",
            "  ▸ Step 2: Detect error signals       ✓",
            "  ▸ Step 3: Determine root cause       ✓",
            "  ▸ Step 4: Plan recovery actions      ◓",
            "  ▸ Step 5: Execute & verify           ○",
        ], "progress", "steps");

        // ─── SPARKLINES / CHARTS ───────────────────────────────────────────────
        add!("chart.sparkline_cpu", &[
            "  CPU:  ▁▂▃▅▆▇█▇▆▅▃▂▁▁▂▃▅▆▇█▇▆▅▃▂▁  (12% avg)",
        ], "chart", "sparkline", "cpu");

        add!("chart.sparkline_mem", &[
            "  MEM:  ▁▂▃▃▄▅▅▆▆▇▇████▇▇▆▆▅▅▄▃▃▂▁  (45% avg)",
        ], "chart", "sparkline", "memory");

        // ─── HELP / UI ELEMENTS ────────────────────────────────────────────────
        add!("ui.keyboard", &[
            "  ┌──────────────────────────────────────────────┐",
            "  │  Keyboard Shortcuts                          │",
            "  ├──────────────┬────────────────────────────────┤",
            "  │  Ctrl+C      │ Exit / Cancel                  │",
            "  │  Ctrl+L      │ Clear screen                   │",
            "  │  Tab         │ Next panel / Autocomplete      │",
            "  │  ↑/↓         │ Navigate history / Select      │",
            "  │  Enter       │ Execute / Confirm              │",
            "  │  ?           │ Context help                   │",
            "  └──────────────┴────────────────────────────────┘",
        ], "ui", "keyboard", "help");

        add!("ui.command_palette", &[
            "  ╔══════════════════════════════════════╗",
            "  ║  ⌘K  Command Palette                  ║",
            "  ╠══════════════════════════════════════╣",
            "  ║  > deploy                          ║",
            "  ║    optidock deploy --strategy=canary║",
            "  ║  > analyze                         ║",
            "  ║    optidock analyze Dockerfile     ║",
            "  ║  > recover                         ║",
            "  ║    optidock agents --watch         ║",
            "  ╚══════════════════════════════════════╝",
        ], "ui", "palette", "command");

        // ─── EMPTY STATES ──────────────────────────────────────────────────────
        add!("empty.no_containers", &[
            "  ┌─────────────────────────────────────┐",
            "  │                                     │",
            "  │        No containers found          │",
            "  │                                     │",
            "  │   Run 'docker compose up' to start  │",
            "  │                                     │",
            "  └─────────────────────────────────────┘",
        ], "empty", "containers");

        add!("empty.no_escalations", &[
            "  ┌─────────────────────────────────────┐",
            "  │                                     │",
            "  │        No active escalations        │",
            "  │                                     │",
            "  │   All containers healthy ✓          │",
            "  │                                     │",
            "  └─────────────────────────────────────┘",
        ], "empty", "escalations");

        // ─── SUCCESS / CELEBRATION ─────────────────────────────────────────────
        add!("celebration.deploy", &[
            "        ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓",
            "      ▓▓  DEPLOY SUCCESSFUL  ▓▓",
            "        ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓",
            "              ████████",
            "           ████████████████",
            "        ██████████████████████",
            "         ████████████████████",
            "          ██████████████████",
            "           ████████████████",
            "             ████████████",
            "               ████████",
        ], "celebration", "deploy", "success");

        add!("celebration.recovery", &[
            "    ✦ ✧ ✦ ✧ ✦ ✧ ✦ ✧ ✦ ✧ ✦ ✧ ✦",
            "   ✧   CONTAINER RECOVERED!    ✧",
            "    ✦ ✧ ✦ ✧ ✦ ✧ ✦ ✧ ✦ ✧ ✦ ✧ ✦",
            "         ┌─────────────┐",
            "         │  ● HEALTHY  │",
            "         └─────────────┘",
        ], "celebration", "recovery", "success");
    }

    /// Get art by key.
    pub fn get(&self, key: &str) -> Option<&AsciiArt> {
        self.art.get(key)
    }

    /// List all registered keys.
    pub fn keys(&self) -> Vec<&'static str> {
        self.art.keys().copied().collect()
    }

    /// Find art by tag.
    pub fn find_by_tag(&self, tag: &str) -> Vec<(&'static str, &AsciiArt)> {
        self.art.iter()
            .filter(|(_, art)| art.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)))
            .map(|(k, v)| (*k, v))
            .collect()
    }
}

/// Convenience: global library instance.
impl ArtLibrary {
    pub fn global() -> &'static ArtLibrary {
        use std::sync::OnceLock;
        static INSTANCE: OnceLock<ArtLibrary> = OnceLock::new();
        INSTANCE.get_or_init(|| ArtLibrary::new())
    }
}

/// Quick accessor for built-in art.
pub fn art(key: &str) -> Option<&'static AsciiArt> {
    ArtLibrary::global().get(key)
}

/// Render art with a simple color function (line index -> colored string).
pub fn render_art(key: &str, style_fn: impl Fn(usize, &str) -> String) -> Option<String> {
    art(key).map(|a| a.render(style_fn))
}

/// Render art plain.
pub fn render_art_plain(key: &str) -> Option<String> {
    art(key).map(|a| a.plain())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_loads_all_art() {
        let lib = ArtLibrary::new();
        assert!(lib.get("brand.logo").is_some());
        assert!(lib.get("docker.whale").is_some());
        assert!(lib.get("recovery.brain").is_some());
    }

    #[test]
    fn art_dimensions_computed() {
        let lib = ArtLibrary::new();
        let logo = lib.get("brand.logo").unwrap();
        assert_eq!(logo.height, 6);
        assert!(logo.width > 40);
    }

    #[test]
    fn tag_search_works() {
        let lib = ArtLibrary::new();
        let docker_art = lib.find_by_tag("docker");
        assert!(!docker_art.is_empty());
    }
}