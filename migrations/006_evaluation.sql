-- ============================================
-- Cleanroom Agent - Evaluation History Schema
-- Version: 006
-- ============================================

-- Evaluation history: stores results of each evaluation run
CREATE TABLE IF NOT EXISTS evaluation_history (
    run_id TEXT PRIMARY KEY,
    project_name TEXT NOT NULL,
    language TEXT NOT NULL,
    version TEXT,
    run_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    duration_ms INTEGER NOT NULL DEFAULT 0,

    -- Analysis quality (Producer)
    file_coverage REAL NOT NULL DEFAULT 0,
    entity_coverage REAL NOT NULL DEFAULT 0,
    type_accuracy REAL,
    f1_score REAL NOT NULL DEFAULT 0,

    -- Generation quality (Consumer)
    compile_pass_rate REAL NOT NULL DEFAULT 0,
    test_pass_rate REAL,
    roundtrip_fidelity REAL NOT NULL DEFAULT 0,

    -- Operational metrics
    files_analyzed INTEGER NOT NULL DEFAULT 0,
    entities_extracted INTEGER NOT NULL DEFAULT 0,
    tasks_completed INTEGER NOT NULL DEFAULT 0,
    tasks_failed INTEGER NOT NULL DEFAULT 0,
    tokens_consumed INTEGER NOT NULL DEFAULT 0,

    -- Full report as JSON for detailed analysis
    report_json TEXT NOT NULL DEFAULT '{}',

    -- Regression tracking
    is_degraded BOOLEAN NOT NULL DEFAULT FALSE,
    degraded_metrics_json TEXT
);

CREATE INDEX IF NOT EXISTS idx_eval_history_project ON evaluation_history(project_name, run_at DESC);
CREATE INDEX IF NOT EXISTS idx_eval_history_degraded ON evaluation_history(is_degraded, run_at);
