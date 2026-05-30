-- ============================================
-- Cleanroom Agent - Agent Collaboration Schema
-- Version: 005
-- ============================================

-- 1. Agent-to-agent messaging queue (docs/13 §3.3)
CREATE TABLE IF NOT EXISTS agent_messages (
    message_id TEXT PRIMARY KEY,
    from_agent TEXT NOT NULL,
    to_agent TEXT NOT NULL,        -- agent_id or "broadcast"
    message_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    read BOOLEAN NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_agent_messages_to ON agent_messages(to_agent, read);

-- 2. Shared type cache across agents (docs/13 §6.1)
CREATE TABLE IF NOT EXISTS type_cache (
    entity_uri TEXT NOT NULL,
    language TEXT NOT NULL,
    resolved_type TEXT NOT NULL,
    source_file TEXT,
    from_lsp BOOLEAN NOT NULL DEFAULT 1,
    cached_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (entity_uri, language)
);

-- 3. Expand agent_type to include reviewer
CREATE TABLE IF NOT EXISTS agents_new (
    agent_id TEXT PRIMARY KEY,
    agent_type TEXT NOT NULL CHECK (agent_type IN ('producer', 'consumer', 'orchestrator', 'reviewer')),
    capabilities_json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'offline'
        CHECK (status IN ('online', 'offline', 'busy')),
    current_task_id TEXT,
    last_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Migrate existing agents to new table (if the old table exists with different constraint)
INSERT OR IGNORE INTO agents_new SELECT * FROM agents;
DROP TABLE IF EXISTS agents;
ALTER TABLE agents_new RENAME TO agents;
