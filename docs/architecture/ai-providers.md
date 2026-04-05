# AI Provider Strategy

OptiDock should never be locked to a single model vendor.

## Supported provider classes

- OpenAI
- Anthropic
- Gemini
- OpenRouter
- local OpenAI-compatible APIs such as LM Studio or vLLM
- Ollama

## Design goals

- keep optimization orchestration provider-agnostic
- isolate API differences in adapters
- support API key based hosted providers
- support local models with no API key
- allow fallback between providers

## Expected environment variables

- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GEMINI_API_KEY`
- `OPENROUTER_API_KEY`

Local endpoints should be configurable with a base URL and model name rather than requiring a key.

## Architectural note

The shared domain should own:

- provider kind
- active provider config
- fallback provider config
- optimization request and proposal types

The agent crate should own:

- provider selection
- fallback logic
- request routing
- structured optimization interfaces
