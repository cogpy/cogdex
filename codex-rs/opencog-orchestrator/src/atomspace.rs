//! Atomspace implementation for knowledge representation and sharing.
//!
//! Inspired by OpenCog's Atomspace, this module provides a way for agents
//! to share knowledge, learn from each other, and maintain context across
//! collaborative coding sessions.

use anyhow::Result;
use chrono::DateTime;
use chrono::Utc;
use indexmap::IndexMap;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

use tokio::sync::RwLock;
use uuid::Uuid;

/// Represents different types of atoms in the knowledge space
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AtomType {
    /// Concept atoms represent abstract ideas or entities
    Concept,
    /// Code atoms represent code snippets, functions, or files
    Code,
    /// Task atoms represent coding tasks or problems
    Task,
    /// Solution atoms represent solutions or approaches
    Solution,
    /// Pattern atoms represent code patterns or templates
    Pattern,
    /// Relationship atoms connect other atoms
    Relationship,
    /// Context atoms provide environmental context
    Context,
    /// Agent atoms represent the specialized agents
    Agent,
}

impl fmt::Display for AtomType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtomType::Concept => write!(f, "concept"),
            AtomType::Code => write!(f, "code"),
            AtomType::Task => write!(f, "task"),
            AtomType::Solution => write!(f, "solution"),
            AtomType::Pattern => write!(f, "pattern"),
            AtomType::Relationship => write!(f, "relationship"),
            AtomType::Context => write!(f, "context"),
            AtomType::Agent => write!(f, "agent"),
        }
    }
}

/// Truth value representation with strength and confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthValue {
    pub strength: Strength,
    pub confidence: Confidence,
}

impl Default for TruthValue {
    fn default() -> Self {
        Self {
            strength: Strength(0.5),
            confidence: Confidence(0.5),
        }
    }
}

impl TruthValue {
    pub fn new(strength: f64, confidence: f64) -> Self {
        Self {
            strength: Strength(strength.clamp(0.0, 1.0)),
            confidence: Confidence(confidence.clamp(0.0, 1.0)),
        }
    }

    pub fn certain(strength: f64) -> Self {
        Self::new(strength, 1.0)
    }

    pub fn uncertain(strength: f64) -> Self {
        Self::new(strength, 0.5)
    }
}

/// Strength of belief (0.0 to 1.0)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Strength(pub f64);

/// Confidence in the strength (0.0 to 1.0)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Confidence(pub f64);

/// Core atom structure in the knowledge space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atom {
    pub id: Uuid,
    pub atom_type: AtomType,
    pub name: String,
    pub content: serde_json::Value,
    pub truth_value: TruthValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub outgoing: Vec<Uuid>, // IDs of atoms this atom links to
}

impl Atom {
    pub fn new(atom_type: AtomType, name: String, content: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            atom_type,
            name,
            content,
            truth_value: TruthValue::default(),
            created_at: now,
            updated_at: now,
            outgoing: Vec::new(),
        }
    }

    pub fn with_truth_value(mut self, truth_value: TruthValue) -> Self {
        self.truth_value = truth_value;
        self
    }

    pub fn with_outgoing(mut self, outgoing: Vec<Uuid>) -> Self {
        self.outgoing = outgoing;
        self
    }

    pub fn update_content(&mut self, content: serde_json::Value) {
        self.content = content;
        self.updated_at = Utc::now();
    }

    pub fn update_truth_value(&mut self, truth_value: TruthValue) {
        self.truth_value = truth_value;
        self.updated_at = Utc::now();
    }
}

/// The Atomspace - central knowledge repository for all agents
pub struct Atomspace {
    atoms: RwLock<IndexMap<Uuid, Atom>>,
    name_index: RwLock<HashMap<String, Vec<Uuid>>>,
    type_index: RwLock<HashMap<AtomType, Vec<Uuid>>>,
}

impl Atomspace {
    pub fn new() -> Self {
        Self {
            atoms: RwLock::new(IndexMap::new()),
            name_index: RwLock::new(HashMap::new()),
            type_index: RwLock::new(HashMap::new()),
        }
    }

    /// Add an atom to the atomspace
    pub async fn add_atom(&self, atom: Atom) -> Result<Uuid> {
        let id = atom.id;
        let name = atom.name.clone();
        let atom_type = atom.atom_type.clone();

        {
            let mut atoms = self.atoms.write().await;
            atoms.insert(id, atom);
        }

        // Update indices
        {
            let mut name_index = self.name_index.write().await;
            name_index.entry(name).or_insert_with(Vec::new).push(id);
        }

        {
            let mut type_index = self.type_index.write().await;
            type_index
                .entry(atom_type)
                .or_insert_with(Vec::new)
                .push(id);
        }

        tracing::debug!("Added atom {id} to atomspace");
        Ok(id)
    }

    /// Get an atom by ID
    pub async fn get_atom(&self, id: Uuid) -> Option<Atom> {
        self.atoms.read().await.get(&id).cloned()
    }

    /// Find atoms by name
    pub async fn find_atoms_by_name(&self, name: &str) -> Vec<Atom> {
        let name_index = self.name_index.read().await;
        if let Some(ids) = name_index.get(name) {
            let atoms = self.atoms.read().await;
            ids.iter().filter_map(|id| atoms.get(id).cloned()).collect()
        } else {
            Vec::new()
        }
    }

    /// Find atoms by type
    pub async fn find_atoms_by_type(&self, atom_type: AtomType) -> Vec<Atom> {
        let type_index = self.type_index.read().await;
        if let Some(ids) = type_index.get(&atom_type) {
            let atoms = self.atoms.read().await;
            ids.iter().filter_map(|id| atoms.get(id).cloned()).collect()
        } else {
            Vec::new()
        }
    }

    /// Update an existing atom
    pub async fn update_atom(&self, id: Uuid, updater: impl FnOnce(&mut Atom)) -> Result<()> {
        let mut atoms = self.atoms.write().await;
        if let Some(atom) = atoms.get_mut(&id) {
            updater(atom);
            tracing::debug!("Updated atom {id}");
            Ok(())
        } else {
            anyhow::bail!("Atom {id} not found")
        }
    }

    /// Remove an atom from the atomspace
    pub async fn remove_atom(&self, id: Uuid) -> Result<()> {
        let atom = {
            let mut atoms = self.atoms.write().await;
            atoms.swap_remove(&id)
        };

        if let Some(atom) = atom {
            // Clean up indices
            {
                let mut name_index = self.name_index.write().await;
                if let Some(ids) = name_index.get_mut(&atom.name) {
                    ids.retain(|&x| x != id);
                    if ids.is_empty() {
                        name_index.remove(&atom.name);
                    }
                }
            }

            {
                let mut type_index = self.type_index.write().await;
                if let Some(ids) = type_index.get_mut(&atom.atom_type) {
                    ids.retain(|&x| x != id);
                    if ids.is_empty() {
                        type_index.remove(&atom.atom_type);
                    }
                }
            }

            tracing::debug!("Removed atom {id}");
            Ok(())
        } else {
            anyhow::bail!("Atom {id} not found")
        }
    }

    /// Get all atoms (for inspection purposes)
    pub async fn get_all_atoms(&self) -> Vec<Atom> {
        self.atoms.read().await.values().cloned().collect()
    }

    /// Get atomspace size
    pub async fn size(&self) -> usize {
        self.atoms.read().await.len()
    }

    /// Create a relationship between atoms
    pub async fn create_relationship(
        &self,
        source: Uuid,
        target: Uuid,
        relation_name: String,
    ) -> Result<Uuid> {
        let relation_atom = Atom::new(
            AtomType::Relationship,
            relation_name,
            serde_json::json!({
                "source": source,
                "target": target
            }),
        )
        .with_outgoing(vec![source, target]);

        self.add_atom(relation_atom).await
    }

    /// Find atoms related to a given atom
    pub async fn find_related_atoms(&self, atom_id: Uuid) -> Vec<Atom> {
        let atoms = self.atoms.read().await;
        atoms
            .values()
            .filter(|atom| {
                atom.outgoing.contains(&atom_id)
                    || (atom.atom_type == AtomType::Relationship
                        && atom.content.get("source")
                            == Some(&serde_json::Value::String(atom_id.to_string()))
                        || atom.content.get("target")
                            == Some(&serde_json::Value::String(atom_id.to_string())))
            })
            .cloned()
            .collect()
    }
}

impl Default for Atomspace {
    fn default() -> Self {
        Self::new()
    }
}
