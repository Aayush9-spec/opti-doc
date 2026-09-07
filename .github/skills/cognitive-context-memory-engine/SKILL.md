---
name: cognitive-context-memory-engine
description: "Use when: designing or implementing a cognitive memory subsystem for an AI agent, including working memory, short-term memory, long-term memory, hybrid retrieval, knowledge graphs, compression, deduplication, forgetting, and observability."
argument-hint: "What memory capability or architecture should be implemented?"
---

# Cognitive Context & Memory Engine

## Role

Build a memory system that behaves like an evolving knowledge substrate rather than a chat log. The engine must continuously transform raw interactions into structured knowledge, retrieve high-value context efficiently, and optimize memory quality under tight token and latency budgets.

## Primary Objectives

- Convert interactions into structured memory objects with intent, decision, evidence, outcome, and confidence.
- Support layered memory with distinct retention, compression, and access semantics.
- Retrieve context through hybrid semantic, symbolic, graph-based, and temporal reasoning.
- Optimize prompt construction under strict token limits.
- Deduplicate, compress, and forget information without losing essential semantics.
- Scale from single-agent usage to distributed multi-agent environments.

## Required Implementation Stack

### Core runtime
- Rust
- Tokio
- Rayon
- SIMD-friendly processing where meaningful
- Async architecture for low-latency concurrent operations

### Retrieval and indexing
- Embedding models such as BGE Large, E5, MiniLM, Jina Embedding, or Nomic Embed
- Vector search backends such as Qdrant, LanceDB, Milvus, Weaviate, FAISS, SQLite Vector, or Chroma
- Similarity search strategies such as HNSW, IVF, PQ, Flat, or DiskANN
- Hybrid retrieval combining dense, sparse, graph, and temporal signals

### Storage and caching
- Metadata storage in PostgreSQL, DuckDB, SQLite, or RocksDB
- Cache layers using Redis, LRU, ARC, TinyLFU, or adaptive cache policies

## Memory Architecture

### 1. Working Memory
- Holds the active reasoning window for the current task.
- Includes current task, files, conversation, goals, code, and plan.
- Stored in RAM only.
- Keep bounded to the last 30 interactions.

### 2. Short-Term Memory
- Stores recent conversation context, active bugs, current architecture, and recent decisions.
- Use a 48-hour TTL.
- Compress automatically on a schedule.

### 3. Long-Term Memory
- Stores durable knowledge such as user preferences, project architecture, coding style, documentation, API knowledge, research, and learned facts.
- Avoid duplication; merge similar memories and increase confidence/frequency instead of creating duplicates.

## Memory Lifecycle

### 1. Capture
Transform each interaction into a structured memory object with:
- Intent
- Decision
- Reason
- Outcome
- Evidence
- Confidence
- Affected files
- Dependencies
- Timestamp

### 2. Normalize and enrich
- Extract entities and relationships.
- Link memories to project, task, API, bug, and architecture concepts.
- Attach metadata such as source, confidence, recency, and access count.

### 3. Store and index
- Embed the memory.
- Index it in the chosen vector store.
- Add graph nodes/edges for relationship-aware retrieval.
- Record metadata in the selected persistent store.

### 4. Retrieve and rank
Use a multi-signal scoring pipeline:
- Semantic similarity
- Graph distance
- Temporal proximity
- Task similarity
- Project match
- User preference match
- Attention score

Prefer a hybrid search pipeline:
1. Embedding search
2. Keyword search
3. Graph traversal
4. Temporal ranking
5. Re-ranking
6. Context assembly

### 5. Compress and deduplicate
- Use hierarchical summarization to compress large memory history into fewer high-value summaries.
- Deduplicate using embedding similarity, MinHash, LSH, and SimHash.
- Merge overlapping memories instead of storing duplicates.

### 6. Forget and archive
- Apply exponential decay to unused memories.
- Promote stale memories to archive state before deletion.
- Only purge memories after they fall below the confidence threshold and provide little value.

## Knowledge Graph Design

Every memory should become a graphable unit with:
- Entity
- Relationship
- Metadata
- Timestamp

Example graph paths:
- User → Project → API → Bug → Fix → Architecture → Research

Use graph methods such as:
- PageRank
- Community detection
- Node2Vec
- Shortest path
- Graph attention or graph neural approaches
- Centrality and connected-component analysis

## Context Construction

Do not send the full memory base to the model. Instead:
1. Define a context budget.
2. Rank memories by importance.
3. Allocate tokens.
4. Compress and pack.
5. Build the final prompt.

Use knapsack-style packing to maximize information gain under token limits rather than relying on first-in-first-out selection.

## Memory Importance Scoring

Each memory should receive a score based on:
- Importance
- Frequency
- Emotional weight
- Task relevance
- Novelty
- Age
- Reference count
- Access count
- Project relation
- Confidence

Suggested formula:
- Importance = 0.25 relevance + 0.20 novelty + 0.15 frequency + 0.15 recency + 0.10 confidence + 0.10 graph_centrality + 0.05 emotional

Apply decay over time unless the memory is referenced again.

## Retrieval Intelligence

Support:
- Approximate nearest neighbor indexing
- Dense-sparse hybrid search
- MMR-based diversity selection
- Contextual bandit or policy-based retrieval improvement
- Re-ranking with cross-encoders or reranker models
- Prediction of likely-needed memories using sequential pattern mining or attention-history heuristics

## Multi-Agent Memory Model

Support memory scopes such as:
- Private memory
- Shared memory
- Project memory
- Organization memory
- Global memory

Each scope should enforce permissions and access rules.

## Security Requirements

- Encrypt memory content using AES-256 or ChaCha20 where appropriate.
- Use secure hashing for integrity and deduplication.
- Apply role-based access controls.
- Protect sensitive memory entries and avoid storing secrets in plaintext.

## Observability

Track:
- Memory hit rate
- Retrieval latency
- Compression ratio
- Duplicate rate
- Graph density
- Embedding drift
- Token efficiency
- Context quality
- Hallucination rate
- Recall
- Precision
- F1
- MRR
- NDCG

## Implementation Workflow

1. Define the memory model and retention policy.
2. Build the core storage abstractions for working, short-term, and long-term memory.
3. Implement ingestion and normalization for structured memories.
4. Add embedding generation, indexing, and retrieval pipelines.
5. Implement knowledge graph construction and graph-based traversal.
6. Add compression, deduplication, and forgetting policies.
7. Build context packing and prompt assembly routines.
8. Add observability, metrics, and memory-quality evaluation.
9. Validate retrieval quality, latency targets, and compression effectiveness.

## Quality Bar

A completed implementation should:
- Store structured knowledge rather than raw chat history.
- Retrieve the most relevant context with hybrid ranking.
- Respect token budgets while maximizing information density.
- Improve over time through feedback and usage patterns.
- Support multi-agent isolation and shared memory safely.
- Maintain low-latency performance under concurrent load.

## Example Prompts

- Implement working, short-term, and long-term memory layers for a Rust agent.
- Add hybrid retrieval with embeddings, keyword search, and graph traversal.
- Build a context packing strategy that selects the highest-value memories under a token budget.
- Add deduplication and forgetting policies for long-lived memory storage.
- Create observability dashboards for retrieval quality and memory lifecycle metrics.
