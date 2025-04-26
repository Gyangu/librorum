use crate::config::{NodeConfig, ClusterConfig};
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_node_config_default() {
    let config = NodeConfig::default();
    assert!(!config.id.is_empty());
    assert_eq!(config.name, "default");
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 50051);
    assert_eq!(config.root_dir, PathBuf::from("."));
    assert_eq!(config.max_file_size, 1024 * 1024 * 1024);
    assert_eq!(config.chunk_size, 1024 * 1024);
    assert!(config.workers > 0);
}

#[test]
fn test_cluster_config_default() {
    let config = ClusterConfig::default();
    assert_eq!(config.sync_interval, 60);
    assert!(config.p2p_enabled);
    assert_eq!(config.nodes.len(), 1);
}

#[test]
fn test_node_config_serialization() {
    let config = NodeConfig::default();
    let serialized = toml::to_string(&config).unwrap();
    let deserialized: NodeConfig = toml::from_str(&serialized).unwrap();
    assert_eq!(config.id, deserialized.id);
    assert_eq!(config.name, deserialized.name);
    assert_eq!(config.host, deserialized.host);
    assert_eq!(config.port, deserialized.port);
    assert_eq!(config.root_dir, deserialized.root_dir);
    assert_eq!(config.max_file_size, deserialized.max_file_size);
    assert_eq!(config.chunk_size, deserialized.chunk_size);
    assert_eq!(config.workers, deserialized.workers);
}

#[test]
fn test_cluster_config_serialization() {
    let config = ClusterConfig::default();
    let serialized = toml::to_string(&config).unwrap();
    let deserialized: ClusterConfig = toml::from_str(&serialized).unwrap();
    assert_eq!(config.nodes.len(), deserialized.nodes.len());
    assert_eq!(config.sync_interval, deserialized.sync_interval);
    assert_eq!(config.p2p_enabled, deserialized.p2p_enabled);
}

#[test]
fn test_node_config_load() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("node.toml");
    
    let config_content = r#"
    [node]
    id = "test-id"
    name = "test-node"
    host = "localhost"
    port = 8080
    root_dir = "."
    max_file_size = 1048576
    chunk_size = 524288
    workers = 4
    "#;
    
    std::fs::write(&config_path, config_content).unwrap();
    
    let config = NodeConfig::load(&config_path).unwrap();
    assert_eq!(config.id, "test-id");
    assert_eq!(config.name, "test-node");
    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 8080);
    assert_eq!(config.max_file_size, 1048576);
    assert_eq!(config.chunk_size, 524288);
    assert_eq!(config.workers, 4);
}

#[test]
fn test_cluster_config_load() {
    let config_content = r#"
sync_interval = 30
p2p_enabled = false
[[nodes]]
id = "node1"
name = "node-1"
host = "localhost"
port = 8080
root_dir = "."
max_file_size = 1048576
chunk_size = 524288
workers = 4
"#;
    let config: ClusterConfig = toml::from_str(config_content).unwrap();
    assert_eq!(config.sync_interval, 30);
    assert_eq!(config.p2p_enabled, false);
    assert_eq!(config.nodes.len(), 1);
    let node = &config.nodes[0];
    assert_eq!(node.id, "node1");
    assert_eq!(node.name, "node-1");
    assert_eq!(node.host, "localhost");
    assert_eq!(node.port, 8080);
    assert_eq!(node.root_dir, PathBuf::from("."));
    assert_eq!(node.max_file_size, 1048576);
    assert_eq!(node.chunk_size, 524288);
    assert_eq!(node.workers, 4);
} 