//! Multi-agent orchestrator for coordinating specialized coding agents.
//!
//! This module implements the main orchestration logic that assigns tasks,
//! coordinates collaboration, and manages the overall workflow of the
//! OpenCog-inspired multi-agent system.

use crate::atomspace::Atom;
use crate::atomspace::AtomType;
use crate::atomspace::Atomspace;
use crate::cognitive_cycle::ActionType;
use crate::cognitive_cycle::CognitiveAction;
use crate::specialized_agents::AgentCapability;
use crate::specialized_agents::CodeAgent;
use crate::specialized_agents::ReviewAgent;
use crate::specialized_agents::SpecializedAgent;
use crate::specialized_agents::TestAgent;
use anyhow::Result;
use codex_core::ConversationManager;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::error;
use tracing::info;
use uuid::Uuid;

/// Configuration for the multi-agent orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Maximum number of agents that can work on a task simultaneously
    pub max_concurrent_agents: usize,
    /// Timeout for individual agent tasks (in seconds)
    pub agent_task_timeout: u64,
    /// Whether to enable automatic collaboration between agents
    pub enable_auto_collaboration: bool,
    /// Minimum confidence required for task assignment
    pub min_task_confidence: f64,
    /// Maximum number of retries for failed tasks
    pub max_retries: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_agents: 3,
            agent_task_timeout: 300, // 5 minutes
            enable_auto_collaboration: true,
            min_task_confidence: 0.6,
            max_retries: 2,
        }
    }
}

/// Represents a task that can be assigned to agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub description: String,
    pub task_type: TaskType,
    pub priority: Priority,
    pub context: HashMap<String, serde_json::Value>,
    pub assigned_agents: Vec<String>,
    pub status: TaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub results: Vec<TaskResult>,
}

/// Types of tasks that can be handled by the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    CodeGeneration,
    CodeReview,
    TestGeneration,
    Debugging,
    Optimization,
    Documentation,
    SystemDesign,
    Collaboration,
}

impl TaskType {
    /// Get the primary capability needed for this task type
    pub fn primary_capability(&self) -> AgentCapability {
        match self {
            TaskType::CodeGeneration => AgentCapability::CodeGeneration,
            TaskType::CodeReview => AgentCapability::CodeReview,
            TaskType::TestGeneration => AgentCapability::TestGeneration,
            TaskType::Debugging => AgentCapability::Debugging,
            TaskType::Optimization => AgentCapability::Optimization,
            TaskType::Documentation => AgentCapability::Documentation,
            TaskType::SystemDesign => AgentCapability::SystemDesign,
            TaskType::Collaboration => AgentCapability::CodeGeneration, // Default
        }
    }
}

/// Task priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Task status tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
    RequiresCollaboration,
}

/// Result from a completed task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub agent_id: String,
    pub result: String,
    pub confidence: f64,
    pub execution_time_ms: u64,
    pub success: bool,
}

/// Task coordinator that manages task lifecycle
pub struct TaskCoordinator {
    tasks: RwLock<HashMap<Uuid, Task>>,
    task_queue: RwLock<Vec<Uuid>>,
}

impl TaskCoordinator {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            task_queue: RwLock::new(Vec::new()),
        }
    }

    /// Add a new task to the system
    pub async fn add_task(&self, mut task: Task) -> Result<Uuid> {
        task.id = Uuid::new_v4();
        task.status = TaskStatus::Pending;
        task.created_at = chrono::Utc::now();

        let task_id = task.id;

        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id, task);
        }

        {
            let mut queue = self.task_queue.write().await;
            queue.push(task_id);
            // Sort by priority (highest first)
            queue.sort_by(|a, b| {
                let tasks_read = self.tasks.try_read();
                if let Ok(tasks) = tasks_read {
                    if let (Some(task_a), Some(task_b)) = (tasks.get(a), tasks.get(b)) {
                        task_b.priority.cmp(&task_a.priority)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                } else {
                    std::cmp::Ordering::Equal
                }
            });
        }

        info!("Added task {} to queue", task_id);
        Ok(task_id)
    }

    /// Get the next pending task
    pub async fn get_next_task(&self) -> Option<Task> {
        let mut queue = self.task_queue.write().await;
        let tasks = self.tasks.read().await;

        while let Some(task_id) = queue.pop() {
            if let Some(task) = tasks.get(&task_id) {
                if task.status == TaskStatus::Pending {
                    return Some(task.clone());
                }
            }
        }

        None
    }

    /// Update task status
    pub async fn update_task_status(&self, task_id: Uuid, status: TaskStatus) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            let is_terminal = matches!(status, TaskStatus::Completed | TaskStatus::Failed);
            task.status = status;
            if is_terminal {
                task.completed_at = Some(chrono::Utc::now());
            }
            Ok(())
        } else {
            anyhow::bail!("Task {} not found", task_id)
        }
    }

    /// Add result to a task
    pub async fn add_task_result(&self, task_id: Uuid, result: TaskResult) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            task.results.push(result);
            Ok(())
        } else {
            anyhow::bail!("Task {} not found", task_id)
        }
    }

    /// Get task by ID
    pub async fn get_task(&self, task_id: Uuid) -> Option<Task> {
        self.tasks.read().await.get(&task_id).cloned()
    }

    /// Get all tasks with a specific status
    pub async fn get_tasks_by_status(&self, status: TaskStatus) -> Vec<Task> {
        self.tasks
            .read()
            .await
            .values()
            .filter(|task| task.status == status)
            .cloned()
            .collect()
    }
}

impl Default for TaskCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// The main multi-agent orchestrator
pub struct MultiAgentOrchestrator {
    config: OrchestratorConfig,
    agents: RwLock<HashMap<String, Box<dyn SpecializedAgent>>>,
    task_coordinator: TaskCoordinator,
    atomspace: Arc<Atomspace>,
    conversation_manager: Arc<ConversationManager>,
}

impl MultiAgentOrchestrator {
    /// Create a new orchestrator with the given configuration
    pub async fn new(
        conversation_manager: Arc<ConversationManager>,
        atomspace: Arc<Atomspace>,
        config: OrchestratorConfig,
    ) -> Result<Self> {
        let mut orchestrator = Self {
            config,
            agents: RwLock::new(HashMap::new()),
            task_coordinator: TaskCoordinator::new(),
            atomspace,
            conversation_manager,
        };

        // Initialize default agents
        orchestrator.initialize_default_agents().await?;

        Ok(orchestrator)
    }

    /// Initialize the default set of specialized agents
    async fn initialize_default_agents(&mut self) -> Result<()> {
        let mut agents = self.agents.write().await;

        // Code Agent
        let code_agent = CodeAgent::new(
            "code_agent_001".to_string(),
            "CodeGen AI".to_string(),
            self.conversation_manager.clone(),
        );
        agents.insert("code_agent_001".to_string(), Box::new(code_agent));

        // Review Agent
        let review_agent = ReviewAgent::new(
            "review_agent_001".to_string(),
            "Review AI".to_string(),
            self.conversation_manager.clone(),
        );
        agents.insert("review_agent_001".to_string(), Box::new(review_agent));

        // Test Agent
        let test_agent = TestAgent::new(
            "test_agent_001".to_string(),
            "Test AI".to_string(),
            self.conversation_manager.clone(),
        );
        agents.insert("test_agent_001".to_string(), Box::new(test_agent));

        info!("Initialized {} specialized agents", agents.len());

        // Add agent atoms to the atomspace
        for agent in agents.values() {
            let agent_atom = Atom::new(
                AtomType::Agent,
                agent.get_name().to_string(),
                serde_json::json!({
                    "id": agent.get_id(),
                    "specializations": agent.get_specializations()
                }),
            );
            self.atomspace.add_atom(agent_atom).await?;
        }

        Ok(())
    }

    /// Start the orchestrator (begins processing tasks)
    pub async fn start(&self) -> Result<()> {
        info!("Starting multi-agent orchestrator");

        // Main orchestration loop
        loop {
            // Process pending tasks
            if let Some(task) = self.task_coordinator.get_next_task().await {
                if let Err(e) = self.process_task(task).await {
                    error!("Error processing task: {e:?}");
                }
            }

            // Process cognitive actions (if any)
            // This would integrate with the cognitive cycle to process recommended actions

            // Small delay to prevent busy loop
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// Coordinate a complex task by breaking it down and assigning to appropriate agents
    pub async fn coordinate_task(&self, task_description: String) -> Result<String> {
        info!("Coordinating task: {}", task_description);

        // Analyze the task to determine type and complexity
        let (task_type, subtasks) = self.analyze_task(&task_description).await?;

        // Create main task
        let main_task = Task {
            id: Uuid::new_v4(),
            description: task_description.clone(),
            task_type: task_type.clone(),
            priority: Priority::High, // Default to high for user requests
            context: HashMap::new(),
            assigned_agents: Vec::new(),
            status: TaskStatus::Pending,
            created_at: chrono::Utc::now(),
            completed_at: None,
            results: Vec::new(),
        };

        let task_id = self.task_coordinator.add_task(main_task).await?;

        // Create and queue subtasks
        let subtask_count = subtasks.len();
        for subtask_desc in subtasks {
            let subtask = Task {
                id: Uuid::new_v4(),
                description: subtask_desc,
                task_type: task_type.clone(),
                priority: Priority::Medium,
                context: {
                    let mut ctx = HashMap::new();
                    ctx.insert(
                        "parent_task".to_string(),
                        serde_json::Value::String(task_id.to_string()),
                    );
                    ctx
                },
                assigned_agents: Vec::new(),
                status: TaskStatus::Pending,
                created_at: chrono::Utc::now(),
                completed_at: None,
                results: Vec::new(),
            };

            self.task_coordinator.add_task(subtask).await?;
        }

        // Store the coordination request in atomspace
        let coordination_atom = Atom::new(
            AtomType::Task,
            format!("coordination_{}", task_id),
            serde_json::json!({
                "main_task": task_id,
                "description": task_description,
                "task_type": task_type,
                "subtask_count": subtask_count
            }),
        );
        self.atomspace.add_atom(coordination_atom).await?;

        // Return immediate acknowledgment
        Ok(format!(
            "Task coordination initiated for: {}\n\
            Task ID: {}\n\
            Subtasks created: {}\n\
            Processing will continue in background.",
            task_description, task_id, subtask_count
        ))
    }

    /// Process a single task by assigning it to the most suitable agent
    async fn process_task(&self, mut task: Task) -> Result<()> {
        info!("Processing task: {} ({})", task.description, task.id);

        // Find the best agent for this task
        let best_agent_id = self.find_best_agent(&task).await?;

        // Assign the task
        task.assigned_agents.push(best_agent_id.clone());
        task.status = TaskStatus::Assigned;
        self.task_coordinator
            .update_task_status(task.id, TaskStatus::Assigned)
            .await?;

        // Execute the task
        let start_time = std::time::Instant::now();
        let agents = self.agents.read().await;

        if let Some(agent) = agents.get(&best_agent_id) {
            self.task_coordinator
                .update_task_status(task.id, TaskStatus::InProgress)
                .await?;

            match agent
                .handle_task(&task.description, &task.context, self.atomspace.clone())
                .await
            {
                Ok(result) => {
                    let execution_time = start_time.elapsed().as_millis() as u64;
                    let task_result = TaskResult {
                        agent_id: best_agent_id.clone(),
                        result: result.clone(),
                        confidence: agent
                            .get_task_confidence(&task.task_type.primary_capability().to_string())
                            .await,
                        execution_time_ms: execution_time,
                        success: true,
                    };

                    self.task_coordinator
                        .add_task_result(task.id, task_result)
                        .await?;
                    self.task_coordinator
                        .update_task_status(task.id, TaskStatus::Completed)
                        .await?;

                    info!(
                        "Task {} completed successfully by {}",
                        task.id, best_agent_id
                    );

                    // Check if collaboration would be beneficial
                    if self.config.enable_auto_collaboration
                        && self.should_request_collaboration(&task, &result).await?
                    {
                        self.initiate_collaboration(task.id).await?;
                    }
                }
                Err(e) => {
                    error!("Task {} failed: {e:?}", task.id);
                    let task_result = TaskResult {
                        agent_id: best_agent_id,
                        result: format!("Error: {e:?}"),
                        confidence: 0.0,
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        success: false,
                    };

                    self.task_coordinator
                        .add_task_result(task.id, task_result)
                        .await?;
                    self.task_coordinator
                        .update_task_status(task.id, TaskStatus::Failed)
                        .await?;
                }
            }
        } else {
            error!("Agent {} not found for task {}", best_agent_id, task.id);
            self.task_coordinator
                .update_task_status(task.id, TaskStatus::Failed)
                .await?;
        }

        Ok(())
    }

    /// Find the best agent for a given task
    async fn find_best_agent(&self, task: &Task) -> Result<String> {
        let agents = self.agents.read().await;
        let primary_capability = task.task_type.primary_capability();

        let mut best_agent_id = None;
        let mut best_score = 0.0;

        for (agent_id, agent) in agents.iter() {
            let mut score = 0.0;

            // Check specializations
            for spec in agent.get_specializations() {
                if spec.capability == primary_capability {
                    score += spec.proficiency * 0.8; // Primary weight
                } else {
                    score += spec.proficiency * 0.2; // Secondary weight
                }
            }

            // Get task-specific confidence
            let task_confidence = agent
                .get_task_confidence(&primary_capability.to_string())
                .await;
            score += task_confidence * 0.5;

            // Prefer agents with fewer current assignments (load balancing)
            // This is simplified - in a real implementation, we'd track active tasks
            score += 0.1;

            if score > best_score && score >= self.config.min_task_confidence {
                best_score = score;
                best_agent_id = Some(agent_id.clone());
            }
        }

        best_agent_id.ok_or_else(|| {
            anyhow::anyhow!(
                "No suitable agent found for task type: {:?}",
                task.task_type
            )
        })
    }

    /// Analyze a task description to determine type and subtasks
    async fn analyze_task(&self, description: &str) -> Result<(TaskType, Vec<String>)> {
        let description_lower = description.to_lowercase();

        let task_type = if description_lower.contains("review")
            || description_lower.contains("audit")
        {
            TaskType::CodeReview
        } else if description_lower.contains("test") {
            TaskType::TestGeneration
        } else if description_lower.contains("debug") || description_lower.contains("fix") {
            TaskType::Debugging
        } else if description_lower.contains("optimize")
            || description_lower.contains("performance")
        {
            TaskType::Optimization
        } else if description_lower.contains("document") {
            TaskType::Documentation
        } else if description_lower.contains("design") || description_lower.contains("architect") {
            TaskType::SystemDesign
        } else {
            TaskType::CodeGeneration // Default
        };

        // Generate subtasks based on the main task
        let subtasks = match task_type {
            TaskType::CodeGeneration => vec![
                "Analyze requirements".to_string(),
                "Design implementation approach".to_string(),
                "Generate core code".to_string(),
                "Add error handling".to_string(),
            ],
            TaskType::CodeReview => vec![
                "Review code quality".to_string(),
                "Check security practices".to_string(),
                "Verify test coverage".to_string(),
                "Provide recommendations".to_string(),
            ],
            TaskType::TestGeneration => vec![
                "Analyze code coverage".to_string(),
                "Generate unit tests".to_string(),
                "Create integration tests".to_string(),
                "Add edge case tests".to_string(),
            ],
            _ => vec![
                "Analyze task requirements".to_string(),
                "Execute primary task".to_string(),
            ],
        };

        Ok((task_type, subtasks))
    }

    /// Determine if collaboration would be beneficial for a task
    async fn should_request_collaboration(&self, task: &Task, result: &str) -> Result<bool> {
        // Simple heuristics for collaboration
        let result_length = result.len();
        let is_complex = result_length > 1000 || task.description.split_whitespace().count() > 20;

        // Check if the task type naturally benefits from collaboration
        let benefits_from_collaboration = matches!(
            task.task_type,
            TaskType::SystemDesign | TaskType::CodeGeneration
        );

        Ok(is_complex && benefits_from_collaboration)
    }

    /// Initiate collaboration between multiple agents for a task
    async fn initiate_collaboration(&self, task_id: Uuid) -> Result<()> {
        info!("Initiating collaboration for task {}", task_id);

        if let Some(task) = self.task_coordinator.get_task(task_id).await {
            // Create a collaboration task
            let collab_task = Task {
                id: Uuid::new_v4(),
                description: format!("Collaborate on: {}", task.description),
                task_type: TaskType::Collaboration,
                priority: task.priority,
                context: {
                    let mut ctx = task.context.clone();
                    ctx.insert(
                        "original_task".to_string(),
                        serde_json::Value::String(task_id.to_string()),
                    );
                    ctx
                },
                assigned_agents: Vec::new(),
                status: TaskStatus::Pending,
                created_at: chrono::Utc::now(),
                completed_at: None,
                results: Vec::new(),
            };

            self.task_coordinator.add_task(collab_task).await?;
        }

        Ok(())
    }

    /// Process cognitive actions from the cognitive cycle
    pub async fn process_cognitive_actions(&self, actions: &[CognitiveAction]) -> Result<()> {
        for action in actions {
            debug!("Processing cognitive action: {:?}", action.action_type);

            match &action.action_type {
                ActionType::SuggestTask { task_description } => {
                    let task = Task {
                        id: Uuid::new_v4(),
                        description: task_description.clone(),
                        task_type: TaskType::CodeGeneration, // Default
                        priority: Priority::Medium,
                        context: {
                            let mut ctx = HashMap::new();
                            ctx.insert(
                                "source".to_string(),
                                serde_json::Value::String("cognitive_cycle".to_string()),
                            );
                            ctx.insert(
                                "confidence".to_string(),
                                serde_json::Value::Number(
                                    serde_json::Number::from_f64(action.confidence)
                                        .unwrap_or(serde_json::Number::from(0)),
                                ),
                            );
                            ctx
                        },
                        assigned_agents: Vec::new(),
                        status: TaskStatus::Pending,
                        created_at: chrono::Utc::now(),
                        completed_at: None,
                        results: Vec::new(),
                    };

                    self.task_coordinator.add_task(task).await?;
                }
                ActionType::RequestCollaboration { agents } => {
                    info!(
                        "Cognitive cycle requested collaboration between agents: {:?}",
                        agents
                    );
                    // Implementation would coordinate specific agent collaboration
                }
                _ => {
                    // Other action types would be handled here
                    debug!("Unhandled cognitive action type: {:?}", action.action_type);
                }
            }
        }

        Ok(())
    }

    /// Get orchestrator statistics
    pub async fn get_stats(&self) -> OrchestratorStats {
        let agents = self.agents.read().await;
        let pending_tasks = self
            .task_coordinator
            .get_tasks_by_status(TaskStatus::Pending)
            .await;
        let in_progress_tasks = self
            .task_coordinator
            .get_tasks_by_status(TaskStatus::InProgress)
            .await;
        let completed_tasks = self
            .task_coordinator
            .get_tasks_by_status(TaskStatus::Completed)
            .await;
        let failed_tasks = self
            .task_coordinator
            .get_tasks_by_status(TaskStatus::Failed)
            .await;

        OrchestratorStats {
            active_agents: agents.len(),
            pending_tasks: pending_tasks.len(),
            in_progress_tasks: in_progress_tasks.len(),
            completed_tasks: completed_tasks.len(),
            failed_tasks: failed_tasks.len(),
            atomspace_size: self.atomspace.size().await,
        }
    }
}

/// Statistics about the orchestrator state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorStats {
    pub active_agents: usize,
    pub pending_tasks: usize,
    pub in_progress_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub atomspace_size: usize,
}
