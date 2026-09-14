use tempfile::tempdir;
use vox_orchestrator::memory::{MemoryConfig, MemoryManager};

#[test]
fn test_sync_verified_research_findings_to_memory_md() {
    let dir = tempdir().unwrap();
    let config = MemoryConfig::for_account("test-acc", dir.path());
    let memory_path = config.memory_md_path.clone();
    let mgr = MemoryManager::new(config).unwrap();

    mgr.sync_verified_research_findings(
        "Rust Tokio vs async-std",
        "Tokio is the industry standard runtime for high-throughput network services.",
        &[(
            "Tokio supports work-stealing multi-threaded scheduler.",
            "Supported",
        )],
    )
    .unwrap();

    let memory_content = std::fs::read_to_string(memory_path).unwrap();
    assert!(memory_content.contains("# Verified Research Knowledgebase"));
    assert!(memory_content.contains("## research:rust-tokio-vs-async-std"));
    assert!(memory_content.contains("Tokio supports work-stealing"));
}
