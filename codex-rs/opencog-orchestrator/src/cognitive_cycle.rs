//! Cognitive cycle implementation for autonomous reasoning and decision making.
//!
//! Implements a simplified version of OpenCog's cognitive cycle that continuously
//! evaluates the atomspace, identifies important atoms, and makes decisions about
//! agent coordination and task prioritization.

use crate::atomspace::Atom;
use crate::atomspace::AtomType;
use crate::atomspace::Atomspace;
use crate::atomspace::TruthValue;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;
use uuid::Uuid;

/// Configuration for the cognitive cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveCycleConfig {
    /// How often to run the cognitive cycle
    pub cycle_interval: Duration,
    /// Maximum number of atoms to consider in each cycle
    pub max_atoms_per_cycle: usize,
    /// Threshold for considering an atom "important"
    pub importance_threshold: f64,
    /// Whether to enable learning from past cycles
    pub enable_learning: bool,
}

impl Default for CognitiveCycleConfig {
    fn default() -> Self {
        Self {
            cycle_interval: Duration::from_secs(5),
            max_atoms_per_cycle: 100,
            importance_threshold: 0.7,
            enable_learning: true,
        }
    }
}

/// Different phases of the cognitive cycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CyclePhase {
    /// Select important atoms for attention
    AtomSelection,
    /// Infer new knowledge from existing atoms
    Inference,
    /// Update truth values based on new evidence
    TruthValueUpdate,
    /// Decide on actions to take
    ActionSelection,
    /// Learn from the cycle outcomes
    Learning,
}

impl std::fmt::Display for CyclePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CyclePhase::AtomSelection => write!(f, "atom_selection"),
            CyclePhase::Inference => write!(f, "inference"),
            CyclePhase::TruthValueUpdate => write!(f, "truth_value_update"),
            CyclePhase::ActionSelection => write!(f, "action_selection"),
            CyclePhase::Learning => write!(f, "learning"),
        }
    }
}

/// Represents a cognitive action that can be taken
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveAction {
    pub action_type: ActionType,
    pub target_atom: Uuid,
    pub confidence: f64,
    pub reasoning: String,
}

/// Types of actions the cognitive system can recommend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    /// Focus more attention on a specific atom/concept
    IncreaseAttention,
    /// Create a new relationship between atoms
    CreateRelationship { target: Uuid },
    /// Update the truth value of an atom
    UpdateTruthValue { new_value: TruthValue },
    /// Suggest a task to an agent
    SuggestTask { task_description: String },
    /// Request collaboration between agents
    RequestCollaboration { agents: Vec<String> },
}

/// The cognitive cycle engine
#[derive(Clone)]
pub struct CognitiveCycle {
    config: CognitiveCycleConfig,
    current_phase: CyclePhase,
    cycle_count: u64,
    recent_actions: Vec<CognitiveAction>,
}

impl CognitiveCycle {
    pub fn new(_atomspace: Arc<Atomspace>, config: CognitiveCycleConfig) -> Self {
        Self {
            config,
            current_phase: CyclePhase::AtomSelection,
            cycle_count: 0,
            recent_actions: Vec::new(),
        }
    }

    /// Run the cognitive cycle continuously
    pub async fn run(&mut self, atomspace: Arc<Atomspace>) -> Result<()> {
        let mut interval = interval(self.config.cycle_interval);
        info!(
            "Starting cognitive cycle with interval {:?}",
            self.config.cycle_interval
        );

        loop {
            interval.tick().await;

            if let Err(e) = self.execute_cycle(atomspace.clone()).await {
                error!("Cognitive cycle error: {e:?}");
            }
        }
    }

    /// Execute one complete cognitive cycle
    async fn execute_cycle(&mut self, atomspace: Arc<Atomspace>) -> Result<()> {
        self.cycle_count += 1;
        debug!("Starting cognitive cycle {}", self.cycle_count);

        // Phase 1: Atom Selection
        self.current_phase = CyclePhase::AtomSelection;
        let important_atoms = self.select_important_atoms(atomspace.clone()).await?;
        trace!("Selected {} important atoms", important_atoms.len());

        // Phase 2: Inference
        self.current_phase = CyclePhase::Inference;
        let new_inferences = self
            .perform_inference(&important_atoms, atomspace.clone())
            .await?;
        trace!("Generated {} new inferences", new_inferences.len());

        // Phase 3: Truth Value Update
        self.current_phase = CyclePhase::TruthValueUpdate;
        self.update_truth_values(&important_atoms, atomspace.clone())
            .await?;

        // Phase 4: Action Selection
        self.current_phase = CyclePhase::ActionSelection;
        let actions = self
            .select_actions(&important_atoms, atomspace.clone())
            .await?;
        self.recent_actions = actions.clone();
        trace!("Selected {} actions", actions.len());

        // Phase 5: Learning (if enabled)
        if self.config.enable_learning {
            self.current_phase = CyclePhase::Learning;
            self.learn_from_cycle(atomspace.clone()).await?;
        }

        debug!("Completed cognitive cycle {}", self.cycle_count);
        Ok(())
    }

    /// Select the most important atoms for attention in this cycle
    async fn select_important_atoms(&self, atomspace: Arc<Atomspace>) -> Result<Vec<Atom>> {
        let all_atoms = atomspace.get_all_atoms().await;
        let mut important_atoms: Vec<Atom> = all_atoms
            .into_iter()
            .filter(|atom| self.calculate_importance(atom) > self.config.importance_threshold)
            .collect();

        // Sort by importance (descending)
        important_atoms.sort_by(|a, b| {
            self.calculate_importance(b)
                .partial_cmp(&self.calculate_importance(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit to max atoms per cycle
        important_atoms.truncate(self.config.max_atoms_per_cycle);

        Ok(important_atoms)
    }

    /// Calculate the importance/attention value of an atom
    fn calculate_importance(&self, atom: &Atom) -> f64 {
        // Base importance on truth value strength and confidence
        let truth_strength = atom.truth_value.strength.0;
        let truth_confidence = atom.truth_value.confidence.0;

        // Consider recency (more recent atoms are more important)
        let age_hours = chrono::Utc::now()
            .signed_duration_since(atom.updated_at)
            .num_hours() as f64;
        let recency_factor = (-age_hours / 24.0).exp(); // Exponential decay over days

        // Consider atom type importance
        let type_importance = match atom.atom_type {
            AtomType::Task => 1.0,
            AtomType::Solution => 0.9,
            AtomType::Code => 0.8,
            AtomType::Agent => 0.7,
            AtomType::Relationship => 0.6,
            AtomType::Pattern => 0.5,
            AtomType::Context => 0.4,
            AtomType::Concept => 0.3,
        };

        // Consider connectivity (atoms with more connections are more important)
        let connectivity_factor = (atom.outgoing.len() as f64).sqrt() * 0.1 + 1.0;

        // Combine factors
        (truth_strength * truth_confidence * type_importance * recency_factor * connectivity_factor)
            .clamp(0.0, 1.0)
    }

    /// Perform inference to derive new knowledge
    async fn perform_inference(
        &self,
        important_atoms: &[Atom],
        atomspace: Arc<Atomspace>,
    ) -> Result<Vec<Atom>> {
        let mut new_atoms = Vec::new();

        // Pattern-based inference: look for common patterns in tasks and solutions
        for atom in important_atoms {
            if atom.atom_type == AtomType::Task {
                if let Some(pattern) = self.infer_task_pattern(atom, atomspace.clone()).await? {
                    new_atoms.push(pattern);
                }
            }
        }

        // Add new atoms to the atomspace
        for atom in &new_atoms {
            atomspace.add_atom(atom.clone()).await?;
        }

        Ok(new_atoms)
    }

    /// Infer patterns from tasks
    async fn infer_task_pattern(
        &self,
        task_atom: &Atom,
        atomspace: Arc<Atomspace>,
    ) -> Result<Option<Atom>> {
        // Look for similar tasks and their solutions
        let related_atoms = atomspace.find_related_atoms(task_atom.id).await;
        let solutions: Vec<_> = related_atoms
            .iter()
            .filter(|a| a.atom_type == AtomType::Solution)
            .collect();

        if solutions.len() >= 2 {
            // Create a pattern atom that captures common elements
            let pattern_content = serde_json::json!({
                "task_type": task_atom.content.get("type"),
                "common_solutions": solutions.len(),
                "confidence_level": "medium",
                "inferred_from": task_atom.id
            });

            let pattern = Atom::new(
                AtomType::Pattern,
                format!("pattern_for_{}", task_atom.name),
                pattern_content,
            )
            .with_truth_value(TruthValue::new(0.6, 0.7))
            .with_outgoing(vec![task_atom.id]);

            trace!("Inferred new pattern from task {}", task_atom.name);
            return Ok(Some(pattern));
        }

        Ok(None)
    }

    /// Update truth values based on new evidence
    async fn update_truth_values(
        &self,
        important_atoms: &[Atom],
        atomspace: Arc<Atomspace>,
    ) -> Result<()> {
        for atom in important_atoms {
            // Simple truth value decay for old atoms
            if chrono::Utc::now()
                .signed_duration_since(atom.updated_at)
                .num_hours()
                > 24
            {
                let decay_factor = 0.95;
                let new_confidence = atom.truth_value.confidence.0 * decay_factor;
                let new_truth_value = TruthValue::new(atom.truth_value.strength.0, new_confidence);

                atomspace
                    .update_atom(atom.id, |a| {
                        a.update_truth_value(new_truth_value);
                    })
                    .await?;
            }
        }

        Ok(())
    }

    /// Select actions to recommend based on current state
    async fn select_actions(
        &self,
        important_atoms: &[Atom],
        _atomspace: Arc<Atomspace>,
    ) -> Result<Vec<CognitiveAction>> {
        let mut actions = Vec::new();

        for atom in important_atoms {
            match atom.atom_type {
                AtomType::Task => {
                    if atom.truth_value.strength.0 > 0.8 {
                        actions.push(CognitiveAction {
                            action_type: ActionType::SuggestTask {
                                task_description: atom.name.clone(),
                            },
                            target_atom: atom.id,
                            confidence: atom.truth_value.confidence.0,
                            reasoning: "High-priority task identified".to_string(),
                        });
                    }
                }
                AtomType::Code => {
                    if atom.truth_value.strength.0 < 0.5 {
                        actions.push(CognitiveAction {
                            action_type: ActionType::IncreaseAttention,
                            target_atom: atom.id,
                            confidence: 0.6,
                            reasoning: "Code atom needs review".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        // Limit actions to prevent overwhelming the system
        actions.truncate(5);
        Ok(actions)
    }

    /// Learn from the outcomes of this cycle
    async fn learn_from_cycle(&mut self, _atomspace: Arc<Atomspace>) -> Result<()> {
        // Simple learning: adjust importance threshold based on recent activity
        let avg_action_confidence = if !self.recent_actions.is_empty() {
            self.recent_actions
                .iter()
                .map(|a| a.confidence)
                .sum::<f64>()
                / self.recent_actions.len() as f64
        } else {
            0.5
        };

        // Adjust threshold slightly based on action quality
        if avg_action_confidence > 0.8 {
            // If we're generating high-confidence actions, we can be more selective
            self.config.importance_threshold = (self.config.importance_threshold + 0.01).min(0.9);
        } else if avg_action_confidence < 0.4 {
            // If actions are low confidence, be less selective
            self.config.importance_threshold = (self.config.importance_threshold - 0.01).max(0.3);
        }

        trace!(
            "Adjusted importance threshold to {}",
            self.config.importance_threshold
        );
        Ok(())
    }

    /// Get the current cognitive cycle statistics
    pub fn get_stats(&self) -> CognitiveStats {
        CognitiveStats {
            cycle_count: self.cycle_count,
            current_phase: self.current_phase,
            recent_actions_count: self.recent_actions.len(),
            importance_threshold: self.config.importance_threshold,
        }
    }

    /// Get recent actions for external systems to process
    pub fn get_recent_actions(&self) -> &[CognitiveAction] {
        &self.recent_actions
    }
}

/// Statistics about the cognitive cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveStats {
    pub cycle_count: u64,
    pub current_phase: CyclePhase,
    pub recent_actions_count: usize,
    pub importance_threshold: f64,
}
