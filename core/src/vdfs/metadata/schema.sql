-- VDFS Metadata Database Schema
-- SQLite schema for persistent metadata storage

-- File metadata table
CREATE TABLE IF NOT EXISTS file_metadata (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    size INTEGER NOT NULL,
    created_timestamp INTEGER NOT NULL,
    modified_timestamp INTEGER NOT NULL,
    accessed_timestamp INTEGER NOT NULL,
    permissions INTEGER NOT NULL,
    checksum TEXT,
    mime_type TEXT,
    is_directory BOOLEAN NOT NULL DEFAULT FALSE,
    version INTEGER NOT NULL DEFAULT 1
);

-- Custom file attributes (key-value pairs)
CREATE TABLE IF NOT EXISTS file_attributes (
    file_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (file_id, key),
    FOREIGN KEY (file_id) REFERENCES file_metadata(id) ON DELETE CASCADE
);

-- Chunk metadata table
CREATE TABLE IF NOT EXISTS chunk_metadata (
    id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    size INTEGER NOT NULL,
    checksum TEXT NOT NULL,
    compressed BOOLEAN NOT NULL DEFAULT FALSE,
    access_count INTEGER NOT NULL DEFAULT 0,
    last_accessed_timestamp INTEGER NOT NULL,
    FOREIGN KEY (file_id) REFERENCES file_metadata(id) ON DELETE CASCADE
);

-- Chunk replica information
CREATE TABLE IF NOT EXISTS chunk_replicas (
    chunk_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    PRIMARY KEY (chunk_id, node_id),
    FOREIGN KEY (chunk_id) REFERENCES chunk_metadata(id) ON DELETE CASCADE
);

-- File replica information
CREATE TABLE IF NOT EXISTS file_replicas (
    file_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    PRIMARY KEY (file_id, node_id),
    FOREIGN KEY (file_id) REFERENCES file_metadata(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_file_path ON file_metadata(path);
CREATE INDEX IF NOT EXISTS idx_file_size ON file_metadata(size);
CREATE INDEX IF NOT EXISTS idx_file_modified ON file_metadata(modified_timestamp);
CREATE INDEX IF NOT EXISTS idx_file_is_directory ON file_metadata(is_directory);
CREATE INDEX IF NOT EXISTS idx_chunk_file_id ON chunk_metadata(file_id);
CREATE INDEX IF NOT EXISTS idx_chunk_access_count ON chunk_metadata(access_count);
CREATE INDEX IF NOT EXISTS idx_file_attributes_key ON file_attributes(key);

-- Full-text search support for file paths and attributes
CREATE VIRTUAL TABLE IF NOT EXISTS file_search USING fts5(
    file_id,
    path,
    attributes,
    content='file_metadata',
    content_rowid='rowid'
);

-- Triggers to maintain FTS index
CREATE TRIGGER IF NOT EXISTS file_search_insert AFTER INSERT ON file_metadata BEGIN
    INSERT INTO file_search(file_id, path, attributes) 
    VALUES (new.id, new.path, '');
END;

CREATE TRIGGER IF NOT EXISTS file_search_delete AFTER DELETE ON file_metadata BEGIN
    DELETE FROM file_search WHERE file_id = old.id;
END;

CREATE TRIGGER IF NOT EXISTS file_search_update AFTER UPDATE ON file_metadata BEGIN
    UPDATE file_search 
    SET path = new.path 
    WHERE file_id = new.id;
END;