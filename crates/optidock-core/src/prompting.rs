use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariable {
    pub key: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPrompt {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: PromptCategory,
    pub system_template: String,
    pub user_template: String,
    pub variables: Vec<PromptVariable>,
    pub temperature_hint: Option<f32>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPack {
    pub prompt_id: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub resolved_variables: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptLibrary {
    pub prompts: Vec<SavedPrompt>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PromptCategory {
    ChatAssistant,
    ArchitectureDesign,
    DockerAnalysis,
    DockerOptimization,
    DeploymentReview,
    Troubleshooting,
    Custom,
}

impl PromptLibrary {
    pub fn new(prompts: Vec<SavedPrompt>) -> Self {
        Self { prompts }
    }

    pub fn get(&self, prompt_id: &str) -> Option<&SavedPrompt> {
        self.prompts.iter().find(|prompt| prompt.id == prompt_id)
    }

    pub fn build_pack(
        &self,
        prompt_id: &str,
        runtime_values: &BTreeMap<String, String>,
    ) -> Option<PromptPack> {
        let prompt = self.get(prompt_id)?;
        Some(prompt.build_pack(runtime_values))
    }
}

impl SavedPrompt {
    pub fn build_pack(&self, runtime_values: &BTreeMap<String, String>) -> PromptPack {
        let resolved_variables = self.resolve_variables(runtime_values);
        let system_prompt = render_template(&self.system_template, &resolved_variables);
        let user_prompt = render_template(&self.user_template, &resolved_variables);

        PromptPack {
            prompt_id: self.id.clone(),
            system_prompt,
            user_prompt,
            resolved_variables,
        }
    }

    fn resolve_variables(
        &self,
        runtime_values: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        let mut resolved = BTreeMap::new();

        for variable in &self.variables {
            if let Some(value) = runtime_values.get(&variable.key) {
                resolved.insert(variable.key.clone(), value.clone());
            } else if let Some(default_value) = &variable.default_value {
                resolved.insert(variable.key.clone(), default_value.clone());
            } else if variable.required {
                resolved.insert(variable.key.clone(), String::new());
            }
        }

        for (key, value) in runtime_values {
            resolved.entry(key.clone()).or_insert_with(|| value.clone());
        }

        resolved
    }
}

fn render_template(template: &str, variables: &BTreeMap<String, String>) -> String {
    let mut rendered = template.to_string();

    for (key, value) in variables {
        let token = format!("{{{{{key}}}}}");
        rendered = rendered.replace(&token, value);
    }

    rendered
}

pub fn default_prompt_library() -> PromptLibrary {
    PromptLibrary::new(vec![
        SavedPrompt {
            id: "chat-default".to_string(),
            name: "Default Chat Assistant".to_string(),
            description: "Balanced assistant behavior for everyday container and developer questions.".to_string(),
            category: PromptCategory::ChatAssistant,
            system_template: "You are OptiDock AI, a terminal-first engineering assistant. Be clear, practical, and concise. Prefer actionable steps over vague advice. Use the available project context when present.".to_string(),
            user_template: "Context:\n{{project_context}}\n\nUser request:\n{{user_input}}\n\nRespond with the best next answer for this operator.".to_string(),
            variables: vec![
                PromptVariable {
                    key: "project_context".to_string(),
                    description: "Project or session context injected before the user request.".to_string(),
                    required: false,
                    default_value: Some("No additional project context provided.".to_string()),
                },
                PromptVariable {
                    key: "user_input".to_string(),
                    description: "The active user message or request.".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            temperature_hint: Some(0.4),
            tags: vec!["chat".to_string(), "default".to_string(), "assistant".to_string()],
        },
        SavedPrompt {
            id: "docker-review".to_string(),
            name: "Dockerfile Review".to_string(),
            description: "Prompt preset for reviewing Dockerfiles and surfacing concrete risks.".to_string(),
            category: PromptCategory::DockerAnalysis,
            system_template: "You are reviewing a Dockerfile like a senior platform engineer. Focus on image size, cache behavior, runtime safety, secret exposure, base image quality, and operational readiness.".to_string(),
            user_template: "Project path: {{project_path}}\n\nKnown findings:\n{{known_findings}}\n\nDockerfile contents:\n{{dockerfile_contents}}\n\nReturn a practical review with clear fixes.".to_string(),
            variables: vec![
                PromptVariable {
                    key: "project_path".to_string(),
                    description: "Repository or project path under review.".to_string(),
                    required: false,
                    default_value: Some("unknown".to_string()),
                },
                PromptVariable {
                    key: "known_findings".to_string(),
                    description: "Deterministic findings discovered before the model call.".to_string(),
                    required: false,
                    default_value: Some("No deterministic findings supplied.".to_string()),
                },
                PromptVariable {
                    key: "dockerfile_contents".to_string(),
                    description: "The Dockerfile being reviewed.".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            temperature_hint: Some(0.2),
            tags: vec!["docker".to_string(), "review".to_string(), "analysis".to_string()],
        },
        SavedPrompt {
            id: "docker-optimize".to_string(),
            name: "Dockerfile Optimizer".to_string(),
            description: "Prompt preset for generating improved Dockerfiles without unnecessary app changes.".to_string(),
            category: PromptCategory::DockerOptimization,
            system_template: "You optimize Dockerfiles for smaller images, faster builds, and safer runtime behavior. Preserve app behavior unless the request explicitly allows deeper changes. Explain assumptions and risks.".to_string(),
            user_template: "Project path: {{project_path}}\n\nConstraints:\n{{constraints}}\n\nFindings:\n{{known_findings}}\n\nCurrent Dockerfile:\n{{dockerfile_contents}}\n\nProduce an improved Dockerfile and a concise justification.".to_string(),
            variables: vec![
                PromptVariable {
                    key: "project_path".to_string(),
                    description: "Repository or project path under optimization.".to_string(),
                    required: false,
                    default_value: Some("unknown".to_string()),
                },
                PromptVariable {
                    key: "constraints".to_string(),
                    description: "Behavioral or operational constraints to preserve.".to_string(),
                    required: false,
                    default_value: Some("Do not change application behavior unless required for container safety.".to_string()),
                },
                PromptVariable {
                    key: "known_findings".to_string(),
                    description: "Analyzer findings already known before optimization.".to_string(),
                    required: false,
                    default_value: Some("No deterministic findings supplied.".to_string()),
                },
                PromptVariable {
                    key: "dockerfile_contents".to_string(),
                    description: "The Dockerfile being optimized.".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            temperature_hint: Some(0.3),
            tags: vec!["docker".to_string(), "optimization".to_string(), "rewrite".to_string()],
        },
        SavedPrompt {
            id: "systems-architecture".to_string(),
            name: "Systems Architecture Expert".to_string(),
            description: "Prompt preset for modular black-box architecture, saved prompts, maintainability, and long-horizon systems design.".to_string(),
            category: PromptCategory::ArchitectureDesign,
            system_template: r#"You are a senior development engineer and systems architect specializing in modular, maintainable software built with black box architecture principles.

Core philosophy:
"It's faster to write five lines of code today than to write one line today and then have to edit it in the future."

You optimize for:
- human cognitive load over cleverness
- replaceable modules
- clean boundaries
- interface-first design
- testable public APIs
- debugging ease
- long-term maintainability

Development rules:
- Hide implementation details and expose only the interface.
- Design APIs first: define what a module does before how it works.
- Prefer clear naming that explains purpose, not mechanism.
- Keep modules single-purpose and small enough for one engineer to understand.
- Avoid leaky abstractions and cross-module internal knowledge.
- Wrap external dependencies behind local interfaces.
- Keep configuration explicit and isolated.
- Prefer simple primitives and composition over complicated data models.

Testing rules:
- Test black-box interfaces, not implementation details.
- Verify integration at module boundaries.
- Design so implementations can be swapped without breaking tests.
- Validate failure handling and interface contracts.
- Make it easy to mock or replace dependencies.

Debugging rules:
- Locate failures at module boundaries first.
- Verify inputs, outputs, and assumptions.
- Recommend boundary logging, replayability, and validation modes when useful.
- Isolate dependency failures from module failures.

When analyzing or designing code:
- identify primitives
- identify black-box boundaries
- recommend clean interfaces
- reduce coupling
- favor replacement-ready architecture
- propose concrete refactoring steps when needed

Your job is to help produce systems that remain understandable and modifiable years from now by different developers, even if the implementation is later replaced completely."#.to_string(),
            user_template: "Project context:\n{{project_context}}\n\nCurrent module or file:\n{{module_context}}\n\nUser request:\n{{user_input}}\n\nRespond as a modular architecture consultant. Focus on boundaries, interfaces, testing surfaces, replacement readiness, and practical implementation guidance.".to_string(),
            variables: vec![
                PromptVariable {
                    key: "project_context".to_string(),
                    description: "High-level project context or system constraints.".to_string(),
                    required: false,
                    default_value: Some("No additional project context provided.".to_string()),
                },
                PromptVariable {
                    key: "module_context".to_string(),
                    description: "The current file, subsystem, or module under discussion.".to_string(),
                    required: false,
                    default_value: Some("No specific module context provided.".to_string()),
                },
                PromptVariable {
                    key: "user_input".to_string(),
                    description: "The architecture or refactoring question to answer.".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            temperature_hint: Some(0.2),
            tags: vec![
                "architecture".to_string(),
                "modularity".to_string(),
                "black-box".to_string(),
                "systems".to_string(),
            ],
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::{default_prompt_library, render_template};
    use std::collections::BTreeMap;

    #[test]
    fn renders_templates_from_variables() {
        let mut values = BTreeMap::new();
        values.insert("name".to_string(), "OptiDock".to_string());

        let rendered = render_template("Hello {{name}}", &values);
        assert_eq!(rendered, "Hello OptiDock");
    }

    #[test]
    fn builds_prompt_pack_with_defaults() {
        let library = default_prompt_library();
        let mut values = BTreeMap::new();
        values.insert("user_input".to_string(), "How do I optimize my Dockerfile?".to_string());

        let pack = library.build_pack("chat-default", &values).unwrap();
        assert!(pack.system_prompt.contains("OptiDock AI"));
        assert!(pack.user_prompt.contains("How do I optimize my Dockerfile?"));
        assert!(pack.user_prompt.contains("No additional project context provided."));
    }

    #[test]
    fn builds_architecture_prompt_pack() {
        let library = default_prompt_library();
        let mut values = BTreeMap::new();
        values.insert(
            "user_input".to_string(),
            "Design a prompt architecture for reusable chatbot presets.".to_string(),
        );

        let pack = library.build_pack("systems-architecture", &values).unwrap();
        assert!(pack.system_prompt.contains("black box architecture principles"));
        assert!(pack.user_prompt.contains("reusable chatbot presets"));
    }
}
