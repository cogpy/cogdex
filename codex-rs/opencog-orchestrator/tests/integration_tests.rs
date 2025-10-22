//! Integration tests for the OpenCog-inspired orchestrator

use anyhow::Result;
use codex_opencog_orchestrator::{Atomspace, AtomType, TruthValue};

#[tokio::test]
async fn test_atomspace_operations() -> Result<()> {
    let atomspace = Atomspace::new();
    
    // Test basic atom creation and retrieval
    let atom = codex_opencog_orchestrator::Atom::new(
        AtomType::Concept,
        "test_concept".to_string(),
        serde_json::json!({"test": "data"}),
    )
    .with_truth_value(TruthValue::new(0.8, 0.9));
    
    let atom_id = atomspace.add_atom(atom.clone()).await?;
    assert_eq!(atom_id, atom.id);
    
    let retrieved = atomspace.get_atom(atom_id).await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "test_concept");
    
    // Test search by type
    let concepts = atomspace.find_atoms_by_type(AtomType::Concept).await;
    assert_eq!(concepts.len(), 1);
    
    // Test search by name
    let named_atoms = atomspace.find_atoms_by_name("test_concept").await;
    assert_eq!(named_atoms.len(), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_cognitive_cycle_stats() -> Result<()> {
    use codex_opencog_orchestrator::{CognitiveCycle, CognitiveCycleConfig};
    use std::sync::Arc;
    
    let atomspace = Arc::new(Atomspace::new());
    let config = CognitiveCycleConfig::default();
    let cycle = CognitiveCycle::new(atomspace, config);
    
    let stats = cycle.get_stats();
    assert_eq!(stats.cycle_count, 0);
    
    Ok(())
}

#[tokio::test]
async fn test_relationship_creation() -> Result<()> {
    let atomspace = Atomspace::new();
    
    // Create two atoms
    let atom1 = codex_opencog_orchestrator::Atom::new(
        AtomType::Concept,
        "concept1".to_string(),
        serde_json::json!({}),
    );
    let atom2 = codex_opencog_orchestrator::Atom::new(
        AtomType::Concept,
        "concept2".to_string(),
        serde_json::json!({}),
    );
    
    let id1 = atomspace.add_atom(atom1).await?;
    let id2 = atomspace.add_atom(atom2).await?;
    
    // Create relationship
    let _rel_id = atomspace.create_relationship(
        id1,
        id2,
        "related_to".to_string(),
    ).await?;
    
    // Verify relationship exists
    let relationships = atomspace.find_atoms_by_type(AtomType::Relationship).await;
    assert_eq!(relationships.len(), 1);
    
    // Find related atoms
    let related = atomspace.find_related_atoms(id1).await;
    assert!(!related.is_empty());
    
    Ok(())
}