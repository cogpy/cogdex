//! OpenCog-inspired multi-agent orchestrator for autonomous coding tasks.
//!
//! This module implements a cognitive architecture that orchestrates multiple
//! specialized coding agents to collaborate on complex development tasks.
//! It draws inspiration from OpenCog's cognitive architecture concepts.

mod atomspace;
mod cognitive_cycle;
mod orchestrator;
mod specialized_agents;

pub use atomspace::Atom;
pub use atomspace::AtomType;
pub use atomspace::Atomspace;
pub use atomspace::Confidence;
pub use atomspace::Strength;
pub use atomspace::TruthValue;
pub use cognitive_cycle::CognitiveCycle;
pub use cognitive_cycle::CognitiveCycleConfig;
pub use cognitive_cycle::CyclePhase;
pub use orchestrator::MultiAgentOrchestrator;
pub use orchestrator::OrchestratorConfig;
pub use orchestrator::TaskCoordinator;
pub use specialized_agents::AgentCapability;
pub use specialized_agents::AgentSpecialization;
pub use specialized_agents::CodeAgent;
pub use specialized_agents::ReviewAgent;
pub use specialized_agents::TestAgent;

use anyhow::Result;
use codex_core::ConversationManager;
use std::sync::Arc;

/// Central facade for the OpenCog-inspired multi-agent system
pub struct OpenCogOrchestrator {
    orchestrator: MultiAgentOrchestrator,
    atomspace: Arc<Atomspace>,
    cognitive_cycle: CognitiveCycle,
}

impl OpenCogOrchestrator {
    /// Create a new OpenCog orchestrator with default configuration
    pub async fn new(conversation_manager: Arc<ConversationManager>) -> Result<Self> {
        let config = OrchestratorConfig::default();
        Self::with_config(conversation_manager, config).await
    }

    /// Create a new OpenCog orchestrator with custom configuration
    pub async fn with_config(
        conversation_manager: Arc<ConversationManager>,
        config: OrchestratorConfig,
    ) -> Result<Self> {
        let atomspace = Arc::new(Atomspace::new());
        let orchestrator =
            MultiAgentOrchestrator::new(conversation_manager, atomspace.clone(), config).await?;

        let cognitive_cycle =
            CognitiveCycle::new(atomspace.clone(), CognitiveCycleConfig::default());

        Ok(Self {
            orchestrator,
            atomspace,
            cognitive_cycle,
        })
    }

    /// Start the cognitive cycle and agent orchestration
    pub async fn start(&mut self) -> Result<()> {
        // Start the cognitive cycle in the background
        let atomspace = self.atomspace.clone();
        let mut cycle = self.cognitive_cycle.clone();

        tokio::spawn(async move {
            if let Err(e) = cycle.run(atomspace).await {
                tracing::error!("Cognitive cycle error: {e:?}");
            }
        });

        // Start the orchestrator
        self.orchestrator.start().await
    }

    /// Submit a complex task to be handled by the multi-agent system
    pub async fn handle_task(&self, task_description: String) -> Result<String> {
        self.orchestrator.coordinate_task(task_description).await
    }

    /// Get the current state of the atomspace for inspection
    pub fn get_atomspace(&self) -> Arc<Atomspace> {
        self.atomspace.clone()
    }
}
