//! Specialized coding agents with different capabilities and focuses.
//!
//! This module defines various types of coding agents that can collaborate
//! on complex development tasks, each with their own specializations and
//! capabilities.

use crate::atomspace::Atom;
use crate::atomspace::AtomType;
use crate::atomspace::Atomspace;
use anyhow::Result;
use async_trait::async_trait;
use codex_core::ConversationManager;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

/// Defines the capabilities an agent can have
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentCapability {
    /// Code generation and implementation
    CodeGeneration,
    /// Code review and quality assurance
    CodeReview,
    /// Test writing and testing strategies
    TestGeneration,
    /// Architecture and design
    SystemDesign,
    /// Documentation writing
    Documentation,
    /// Performance optimization
    Optimization,
    /// Security analysis
    Security,
    /// Bug fixing and debugging
    Debugging,
    /// API design and integration
    ApiDesign,
    /// Database design and queries
    DatabaseDesign,
}

impl std::fmt::Display for AgentCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentCapability::CodeGeneration => write!(f, "code_generation"),
            AgentCapability::CodeReview => write!(f, "code_review"),
            AgentCapability::TestGeneration => write!(f, "test_generation"),
            AgentCapability::SystemDesign => write!(f, "system_design"),
            AgentCapability::Documentation => write!(f, "documentation"),
            AgentCapability::Optimization => write!(f, "optimization"),
            AgentCapability::Security => write!(f, "security"),
            AgentCapability::Debugging => write!(f, "debugging"),
            AgentCapability::ApiDesign => write!(f, "api_design"),
            AgentCapability::DatabaseDesign => write!(f, "database_design"),
        }
    }
}

/// Agent specialization with proficiency levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpecialization {
    pub capability: AgentCapability,
    pub proficiency: f64,        // 0.0 to 1.0
    pub languages: Vec<String>,  // Programming languages
    pub frameworks: Vec<String>, // Frameworks and libraries
}

impl AgentSpecialization {
    pub fn new(capability: AgentCapability, proficiency: f64) -> Self {
        Self {
            capability,
            proficiency: proficiency.clamp(0.0, 1.0),
            languages: Vec::new(),
            frameworks: Vec::new(),
        }
    }

    pub fn with_languages(mut self, languages: Vec<String>) -> Self {
        self.languages = languages;
        self
    }

    pub fn with_frameworks(mut self, frameworks: Vec<String>) -> Self {
        self.frameworks = frameworks;
        self
    }
}

/// Base trait for all specialized agents
#[async_trait]
pub trait SpecializedAgent: Send + Sync {
    /// Get the agent's unique identifier
    fn get_id(&self) -> &str;

    /// Get the agent's name
    fn get_name(&self) -> &str;

    /// Get the agent's specializations
    fn get_specializations(&self) -> &[AgentSpecialization];

    /// Handle a specific task assigned to this agent
    async fn handle_task(
        &self,
        task: &str,
        context: &HashMap<String, serde_json::Value>,
        atomspace: Arc<Atomspace>,
    ) -> Result<String>;

    /// Collaborate with another agent on a task
    async fn collaborate(
        &self,
        other_agent_id: &str,
        task: &str,
        _shared_context: &HashMap<String, serde_json::Value>,
        atomspace: Arc<Atomspace>,
    ) -> Result<String>;

    /// Get confidence level for handling a specific task type
    async fn get_task_confidence(&self, task_type: &str) -> f64;

    /// Update the agent's knowledge based on feedback
    async fn learn_from_feedback(
        &mut self,
        task: &str,
        outcome: &str,
        success: bool,
        atomspace: Arc<Atomspace>,
    ) -> Result<()>;
}

/// Code generation and implementation agent
pub struct CodeAgent {
    id: String,
    name: String,
    specializations: Vec<AgentSpecialization>,
    conversation_manager: Arc<ConversationManager>,
    task_history: HashMap<String, f64>, // task -> success rate
}

impl CodeAgent {
    pub fn new(id: String, name: String, conversation_manager: Arc<ConversationManager>) -> Self {
        let specializations = vec![
            AgentSpecialization::new(AgentCapability::CodeGeneration, 0.9)
                .with_languages(vec![
                    "rust".to_string(),
                    "python".to_string(),
                    "typescript".to_string(),
                    "javascript".to_string(),
                ])
                .with_frameworks(vec![
                    "tokio".to_string(),
                    "axum".to_string(),
                    "serde".to_string(),
                ]),
            AgentSpecialization::new(AgentCapability::SystemDesign, 0.7),
            AgentSpecialization::new(AgentCapability::ApiDesign, 0.8),
        ];

        Self {
            id,
            name,
            specializations,
            conversation_manager,
            task_history: HashMap::new(),
        }
    }
}

#[async_trait]
impl SpecializedAgent for CodeAgent {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_specializations(&self) -> &[AgentSpecialization] {
        &self.specializations
    }

    async fn handle_task(
        &self,
        task: &str,
        context: &HashMap<String, serde_json::Value>,
        atomspace: Arc<Atomspace>,
    ) -> Result<String> {
        info!("CodeAgent {} handling task: {}", self.name, task);

        // Store task in atomspace
        let task_atom = Atom::new(
            AtomType::Task,
            format!("task_{}", Uuid::new_v4()),
            serde_json::json!({
                "description": task,
                "agent": self.id,
                "type": "code_generation",
                "context": context
            }),
        );
        let task_id = atomspace.add_atom(task_atom).await?;

        // Simulate code generation (in a real implementation, this would
        // interface with the conversation manager to generate actual code)
        let result = format!(
            "Generated code for task: {}\n\
            // Implementation details would be generated here\n\
            // Context: {:#?}",
            task, context
        );

        // Store result in atomspace
        let solution_atom = Atom::new(
            AtomType::Solution,
            format!("solution_{}", Uuid::new_v4()),
            serde_json::json!({
                "task_id": task_id,
                "agent": self.id,
                "solution": result,
                "confidence": self.get_task_confidence("code_generation").await
            }),
        )
        .with_outgoing(vec![task_id]);

        atomspace.add_atom(solution_atom).await?;

        debug!(
            "CodeAgent completed task with result length: {}",
            result.len()
        );
        Ok(result)
    }

    async fn collaborate(
        &self,
        other_agent_id: &str,
        task: &str,
        _shared_context: &HashMap<String, serde_json::Value>,
        atomspace: Arc<Atomspace>,
    ) -> Result<String> {
        info!(
            "CodeAgent {} collaborating with {} on: {}",
            self.name, other_agent_id, task
        );

        // Create a collaborative relationship in the atomspace
        let collab_atom = Atom::new(
            AtomType::Relationship,
            format!("collaboration_{}_{}", self.id, other_agent_id),
            serde_json::json!({
                "type": "collaboration",
                "agents": [self.id, other_agent_id],
                "task": task,
                "context": _shared_context
            }),
        );
        atomspace.add_atom(collab_atom).await?;

        // Generate collaborative solution
        let result = format!(
            "Collaborative code solution between {} and {}:\n\
            Task: {}\n\
            // Code would be generated here with input from both agents",
            self.name, other_agent_id, task
        );

        Ok(result)
    }

    async fn get_task_confidence(&self, task_type: &str) -> f64 {
        match task_type {
            "code_generation" => 0.9,
            "system_design" => 0.7,
            "api_design" => 0.8,
            _ => 0.3,
        }
    }

    async fn learn_from_feedback(
        &mut self,
        task: &str,
        _outcome: &str,
        success: bool,
        atomspace: Arc<Atomspace>,
    ) -> Result<()> {
        let current_rate = self.task_history.get(task).unwrap_or(&0.5);
        let new_rate = if success {
            (current_rate + 0.1).min(1.0)
        } else {
            (current_rate - 0.05).max(0.0)
        };

        self.task_history.insert(task.to_string(), new_rate);

        // Store learning in atomspace
        let learning_atom = Atom::new(
            AtomType::Context,
            format!("learning_{}_{}", self.id, Uuid::new_v4()),
            serde_json::json!({
                "agent": self.id,
                "task": task,
                "success": success,
                "new_success_rate": new_rate
            }),
        );
        atomspace.add_atom(learning_atom).await?;

        debug!("CodeAgent learned from feedback: {} -> {}", task, new_rate);
        Ok(())
    }
}

/// Code review and quality assurance agent
pub struct ReviewAgent {
    id: String,
    name: String,
    specializations: Vec<AgentSpecialization>,
    conversation_manager: Arc<ConversationManager>,
    review_standards: HashMap<String, f64>, // standard -> threshold
}

impl ReviewAgent {
    pub fn new(id: String, name: String, conversation_manager: Arc<ConversationManager>) -> Self {
        let specializations = vec![
            AgentSpecialization::new(AgentCapability::CodeReview, 0.95).with_languages(vec![
                "rust".to_string(),
                "python".to_string(),
                "typescript".to_string(),
            ]),
            AgentSpecialization::new(AgentCapability::Security, 0.8),
            AgentSpecialization::new(AgentCapability::Optimization, 0.7),
        ];

        let mut review_standards = HashMap::new();
        review_standards.insert("code_quality".to_string(), 0.8);
        review_standards.insert("security".to_string(), 0.9);
        review_standards.insert("performance".to_string(), 0.7);

        Self {
            id,
            name,
            specializations,
            conversation_manager,
            review_standards,
        }
    }
}

#[async_trait]
impl SpecializedAgent for ReviewAgent {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_specializations(&self) -> &[AgentSpecialization] {
        &self.specializations
    }

    async fn handle_task(
        &self,
        task: &str,
        context: &HashMap<String, serde_json::Value>,
        atomspace: Arc<Atomspace>,
    ) -> Result<String> {
        info!("ReviewAgent {} reviewing task: {}", self.name, task);

        // Find related code atoms to review
        let code_atoms = atomspace.find_atoms_by_type(AtomType::Code).await;
        let mut review_results = Vec::new();

        for code_atom in code_atoms.iter().take(5) {
            // Limit review scope
            let quality_score = self.assess_code_quality(&code_atom.content).await;
            let security_score = self.assess_security(&code_atom.content).await;

            review_results.push(serde_json::json!({
                "atom_id": code_atom.id,
                "quality_score": quality_score,
                "security_score": security_score,
                "recommendations": self.generate_recommendations(quality_score, security_score)
            }));
        }

        let review_atom = Atom::new(
            AtomType::Solution,
            format!("review_{}", Uuid::new_v4()),
            serde_json::json!({
                "task": task,
                "agent": self.id,
                "reviews": review_results,
                "context": context
            }),
        );
        atomspace.add_atom(review_atom).await?;

        let result = format!(
            "Code review completed by {}:\n\
            Reviewed {} code items\n\
            Results: {:#?}",
            self.name,
            code_atoms.len(),
            review_results
        );

        Ok(result)
    }

    async fn collaborate(
        &self,
        other_agent_id: &str,
        task: &str,
        _shared_context: &HashMap<String, serde_json::Value>,
        atomspace: Arc<Atomspace>,
    ) -> Result<String> {
        info!(
            "ReviewAgent {} collaborating with {} on: {}",
            self.name, other_agent_id, task
        );

        // In collaboration, provide review feedback to the other agent
        let feedback = format!(
            "Review feedback for collaboration with {}:\n\
            Task: {}\n\
            Recommendations: Focus on code quality and security standards",
            other_agent_id, task
        );

        let collab_atom = Atom::new(
            AtomType::Relationship,
            format!("review_collaboration_{}_{}", self.id, other_agent_id),
            serde_json::json!({
                "type": "review_collaboration",
                "reviewer": self.id,
                "reviewee": other_agent_id,
                "task": task,
                "feedback": feedback
            }),
        );
        atomspace.add_atom(collab_atom).await?;

        Ok(feedback)
    }

    async fn get_task_confidence(&self, task_type: &str) -> f64 {
        match task_type {
            "code_review" => 0.95,
            "security" => 0.8,
            "optimization" => 0.7,
            _ => 0.2,
        }
    }

    async fn learn_from_feedback(
        &mut self,
        task: &str,
        outcome: &str,
        success: bool,
        atomspace: Arc<Atomspace>,
    ) -> Result<()> {
        if success {
            info!("ReviewAgent learned from successful review: {}", task);
        } else {
            warn!("ReviewAgent learning from failed review: {}", task);
            // Adjust review standards if needed
            for (standard, threshold) in &mut self.review_standards {
                if outcome.contains(standard) {
                    *threshold = (*threshold * 0.95).max(0.5); // Lower threshold slightly
                }
            }
        }

        let learning_atom = Atom::new(
            AtomType::Context,
            format!("review_learning_{}_{}", self.id, Uuid::new_v4()),
            serde_json::json!({
                "agent": self.id,
                "task": task,
                "outcome": outcome,
                "success": success,
                "updated_standards": self.review_standards
            }),
        );
        atomspace.add_atom(learning_atom).await?;

        Ok(())
    }
}

impl ReviewAgent {
    async fn assess_code_quality(&self, code_content: &serde_json::Value) -> f64 {
        // Simplified quality assessment
        if let Some(code_str) = code_content.as_str() {
            let mut score: f64 = 0.7; // Base score

            if code_str.contains("// ") || code_str.contains("/// ") {
                score += 0.1; // Has comments
            }
            if code_str.contains("test") {
                score += 0.1; // Has tests
            }
            if code_str.len() > 1000 {
                score -= 0.1; // Too long
            }

            score.clamp(0.0, 1.0)
        } else {
            0.5
        }
    }

    async fn assess_security(&self, code_content: &serde_json::Value) -> f64 {
        // Simplified security assessment
        if let Some(code_str) = code_content.as_str() {
            let mut score: f64 = 0.8; // Base score

            if code_str.contains("unsafe") {
                score -= 0.3; // Unsafe code
            }
            if code_str.contains("unwrap()") {
                score -= 0.1; // Potential panic
            }
            if code_str.contains("expect(") {
                score += 0.1; // Better error handling
            }

            score.clamp(0.0, 1.0)
        } else {
            0.5
        }
    }

    fn generate_recommendations(&self, quality_score: f64, security_score: f64) -> Vec<String> {
        let mut recommendations = Vec::new();

        if quality_score < 0.7 {
            recommendations.push("Improve code documentation and structure".to_string());
        }
        if security_score < 0.7 {
            recommendations.push("Review security practices and error handling".to_string());
        }
        if quality_score > 0.9 && security_score > 0.9 {
            recommendations.push("Excellent code quality - ready for production".to_string());
        }

        recommendations
    }
}

/// Test generation and testing strategy agent
pub struct TestAgent {
    id: String,
    name: String,
    specializations: Vec<AgentSpecialization>,
    conversation_manager: Arc<ConversationManager>,
    test_strategies: Vec<String>,
}

impl TestAgent {
    pub fn new(id: String, name: String, conversation_manager: Arc<ConversationManager>) -> Self {
        let specializations = vec![
            AgentSpecialization::new(AgentCapability::TestGeneration, 0.9)
                .with_languages(vec![
                    "rust".to_string(),
                    "python".to_string(),
                    "typescript".to_string(),
                ])
                .with_frameworks(vec![
                    "tokio-test".to_string(),
                    "pytest".to_string(),
                    "jest".to_string(),
                ]),
            AgentSpecialization::new(AgentCapability::Debugging, 0.8),
        ];

        let test_strategies = vec![
            "unit_tests".to_string(),
            "integration_tests".to_string(),
            "property_based_tests".to_string(),
            "snapshot_tests".to_string(),
            "performance_tests".to_string(),
        ];

        Self {
            id,
            name,
            specializations,
            conversation_manager,
            test_strategies,
        }
    }
}

#[async_trait]
impl SpecializedAgent for TestAgent {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_specializations(&self) -> &[AgentSpecialization] {
        &self.specializations
    }

    async fn handle_task(
        &self,
        task: &str,
        context: &HashMap<String, serde_json::Value>,
        atomspace: Arc<Atomspace>,
    ) -> Result<String> {
        info!("TestAgent {} generating tests for: {}", self.name, task);

        // Find code atoms to generate tests for
        let code_atoms = atomspace.find_atoms_by_type(AtomType::Code).await;
        let mut test_plans = Vec::new();

        for code_atom in code_atoms.iter().take(3) {
            let test_plan = self.generate_test_plan(&code_atom.content).await;
            test_plans.push(serde_json::json!({
                "target_code": code_atom.id,
                "test_plan": test_plan,
                "strategies": self.test_strategies
            }));
        }

        let test_atom = Atom::new(
            AtomType::Solution,
            format!("tests_{}", Uuid::new_v4()),
            serde_json::json!({
                "task": task,
                "agent": self.id,
                "test_plans": test_plans,
                "context": context
            }),
        );
        atomspace.add_atom(test_atom).await?;

        let result = format!(
            "Test generation completed by {}:\n\
            Generated test plans for {} code items\n\
            Strategies: {:#?}",
            self.name,
            code_atoms.len(),
            self.test_strategies
        );

        Ok(result)
    }

    async fn collaborate(
        &self,
        other_agent_id: &str,
        task: &str,
        _shared_context: &HashMap<String, serde_json::Value>,
        atomspace: Arc<Atomspace>,
    ) -> Result<String> {
        info!(
            "TestAgent {} collaborating with {} on: {}",
            self.name, other_agent_id, task
        );

        let collaboration = format!(
            "Test collaboration with {}:\n\
            Will provide comprehensive test coverage for code generated\n\
            Task: {}",
            other_agent_id, task
        );

        let collab_atom = Atom::new(
            AtomType::Relationship,
            format!("test_collaboration_{}_{}", self.id, other_agent_id),
            serde_json::json!({
                "type": "test_collaboration",
                "tester": self.id,
                "developer": other_agent_id,
                "task": task,
                "strategy": "comprehensive_coverage"
            }),
        );
        atomspace.add_atom(collab_atom).await?;

        Ok(collaboration)
    }

    async fn get_task_confidence(&self, task_type: &str) -> f64 {
        match task_type {
            "test_generation" => 0.9,
            "debugging" => 0.8,
            "code_analysis" => 0.7,
            _ => 0.3,
        }
    }

    async fn learn_from_feedback(
        &mut self,
        task: &str,
        outcome: &str,
        success: bool,
        atomspace: Arc<Atomspace>,
    ) -> Result<()> {
        if success {
            debug!("TestAgent learned from successful testing: {}", task);
        } else {
            warn!("TestAgent learning from testing issues: {}", task);
            // Maybe adjust test strategies based on outcome
        }

        let learning_atom = Atom::new(
            AtomType::Context,
            format!("test_learning_{}_{}", self.id, Uuid::new_v4()),
            serde_json::json!({
                "agent": self.id,
                "task": task,
                "outcome": outcome,
                "success": success
            }),
        );
        atomspace.add_atom(learning_atom).await?;

        Ok(())
    }
}

impl TestAgent {
    async fn generate_test_plan(&self, code_content: &serde_json::Value) -> Vec<String> {
        // Simplified test plan generation
        let mut plan = vec![
            "Generate unit tests for core functions".to_string(),
            "Add edge case testing".to_string(),
        ];

        if let Some(code_str) = code_content.as_str() {
            if code_str.contains("async") {
                plan.push("Add async/await testing".to_string());
            }
            if code_str.contains("Result") || code_str.contains("Option") {
                plan.push("Test error handling paths".to_string());
            }
            if code_str.contains("pub struct") {
                plan.push("Add property-based tests".to_string());
            }
        }

        plan
    }
}
