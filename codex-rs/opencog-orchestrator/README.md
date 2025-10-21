# OpenCog Multi-Agent Orchestrator

An OpenCog-inspired cognitive architecture implementation for autonomous coding multi-agent orchestration, integrated with the Codex CLI system.

## Overview

This module provides:

- **Atomspace**: A knowledge representation system for sharing information between agents
- **Cognitive Cycle**: Autonomous reasoning and decision-making engine
- **Specialized Agents**: Multiple coding agents with different capabilities
- **Multi-Agent Orchestrator**: Coordination system for complex coding tasks

## Architecture

### Core Components

1. **Atomspace** (`atomspace.rs`)
   - Central knowledge repository
   - Stores atoms with truth values and relationships
   - Enables knowledge sharing between agents
   - Supports queries by type, name, and relationships

2. **Cognitive Cycle** (`cognitive_cycle.rs`) 
   - Continuously evaluates atomspace content
   - Makes autonomous decisions about task prioritization
   - Learns from past experiences
   - Recommends actions to the orchestrator

3. **Specialized Agents** (`specialized_agents.rs`)
   - **CodeAgent**: Code generation and implementation
   - **ReviewAgent**: Code review and quality assurance  
   - **TestAgent**: Test generation and testing strategies
   - Each agent has specific capabilities and proficiencies

4. **Multi-Agent Orchestrator** (`orchestrator.rs`)
   - Coordinates task assignment and execution
   - Manages agent collaboration
   - Handles task lifecycle and monitoring
   - Integrates with Codex conversation system

## Usage

### Basic Setup

```rust
use codex_opencog_orchestrator::OpenCogOrchestrator;
use codex_core::ConversationManager;
use std::sync::Arc;

// Create with default configuration
let conversation_manager: Arc<dyn ConversationManager> = // ... obtain from Codex
let orchestrator = OpenCogOrchestrator::new(conversation_manager).await?;

// Start the cognitive system
orchestrator.start().await?;

// Submit a complex task
let result = orchestrator.handle_task(
    "Implement a new REST API endpoint with validation and tests".to_string()
).await?;
```

### Custom Configuration

```rust
use codex_opencog_orchestrator::{OpenCogOrchestrator, OrchestratorConfig};

let config = OrchestratorConfig {
    max_concurrent_agents: 5,
    agent_task_timeout: 600, // 10 minutes
    enable_auto_collaboration: true,
    min_task_confidence: 0.7,
    max_retries: 3,
};

let orchestrator = OpenCogOrchestrator::with_config(
    conversation_manager, 
    config
).await?;
```

### Atomspace Interaction

```rust
use codex_opencog_orchestrator::{Atom, AtomType, TruthValue};

let atomspace = orchestrator.get_atomspace();

// Create knowledge atoms
let code_atom = Atom::new(
    AtomType::Code,
    "user_authentication".to_string(),
    serde_json::json!({
        "language": "rust",
        "complexity": "medium",
        "status": "implemented"
    })
).with_truth_value(TruthValue::certain(0.9));

atomspace.add_atom(code_atom).await?;

// Query atoms
let code_atoms = atomspace.find_atoms_by_type(AtomType::Code).await;
let auth_atoms = atomspace.find_atoms_by_name("user_authentication").await;
```

## Features

### Autonomous Task Coordination

- Automatically breaks down complex tasks into subtasks
- Assigns tasks to most suitable agents based on capabilities
- Monitors task progress and handles failures
- Enables automatic collaboration when beneficial

### Cognitive Decision Making

- Continuously evaluates importance of knowledge atoms
- Makes autonomous decisions about task priorities
- Learns from past successes and failures
- Adapts behavior over time

### Specialized Agent Capabilities

| Agent | Primary Capabilities | Secondary Capabilities |
|-------|---------------------|----------------------|
| CodeAgent | Code Generation (90%) | System Design (70%), API Design (80%) |
| ReviewAgent | Code Review (95%) | Security (80%), Optimization (70%) |
| TestAgent | Test Generation (90%) | Debugging (80%) |

### Knowledge Representation

- **Truth Values**: Each atom has strength and confidence values
- **Relationships**: Atoms can be connected via typed relationships  
- **Temporal Decay**: Old knowledge gradually loses confidence
- **Pattern Learning**: System identifies recurring patterns

## Integration with Codex

The orchestrator integrates seamlessly with the existing Codex CLI system:

- Uses `ConversationManager` for actual code generation
- Leverages Codex's MCP (Model Context Protocol) capabilities
- Respects Codex's sandboxing and security constraints
- Stores results in Codex conversation history

## Configuration

The system can be configured through:

1. **OrchestratorConfig**: Main orchestrator settings
2. **CognitiveCycleConfig**: Cognitive reasoning parameters  
3. **Agent Specializations**: Individual agent capabilities

See the `Config` structs for full configuration options.

## Testing

Run tests with:

```bash
cargo test -p codex-opencog-orchestrator
```

Integration tests cover:
- Atomspace operations and queries
- Cognitive cycle statistics
- Relationship creation and traversal
- Agent task handling (simulated)

## Future Enhancements

- Additional specialized agents (DocAgent, SecurityAgent, etc.)
- Advanced cognitive reasoning algorithms
- Machine learning integration for agent improvement
- Real-time collaboration monitoring
- Performance optimization through atomspace indexing

## License

Licensed under Apache-2.0, same as the main Codex project.