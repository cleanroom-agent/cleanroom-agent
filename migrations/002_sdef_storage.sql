-- ============================================
-- Cleanroom Agent - S.DEF Extended Storage
-- Version: 002
-- Extends storage to cover Architecture, Domain,
-- Behavior (flows, state machines), UI Navigation,
-- Responsive Design, Deployment, and Dependencies.
-- ============================================

-- ============================================
-- 1. 架构层 (Architecture)
-- ============================================

CREATE TABLE architecture_docs (
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    style TEXT,
    rationale TEXT,
    PRIMARY KEY (document_name)
);

CREATE TABLE architecture_layers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    layer_name TEXT NOT NULL,
    components_json TEXT
);

CREATE TABLE architecture_modules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    module_name TEXT NOT NULL,
    responsibility TEXT,
    exports_json TEXT,
    depends_on_json TEXT
);

CREATE TABLE module_components (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    module_id INTEGER NOT NULL REFERENCES architecture_modules(id) ON DELETE CASCADE,
    component_name TEXT NOT NULL,
    component_type TEXT NOT NULL,
    description TEXT
);

CREATE TABLE communication_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    pattern_type TEXT NOT NULL,
    description TEXT
);

CREATE TABLE cross_cutting_concerns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    concern_name TEXT NOT NULL,
    description TEXT NOT NULL
);

-- ============================================
-- 2. 领域层 (Domain)
-- ============================================

CREATE TABLE business_concepts (
    concept_id TEXT NOT NULL,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    invariants_json TEXT,
    PRIMARY KEY (document_name, concept_id)
);

CREATE TABLE concept_attributes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL,
    concept_id TEXT NOT NULL,
    attr_name TEXT NOT NULL,
    attr_type TEXT NOT NULL,
    description TEXT,
    domain TEXT,
    FOREIGN KEY (document_name, concept_id) REFERENCES business_concepts(document_name, concept_id) ON DELETE CASCADE
);

CREATE TABLE concept_relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL,
    concept_id TEXT NOT NULL,
    role TEXT NOT NULL,
    target TEXT NOT NULL,
    cardinality TEXT,
    FOREIGN KEY (document_name, concept_id) REFERENCES business_concepts(document_name, concept_id) ON DELETE CASCADE
);

CREATE TABLE business_rules (
    rule_id TEXT NOT NULL,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    condition TEXT NOT NULL,
    action TEXT NOT NULL,
    priority TEXT,
    PRIMARY KEY (document_name, rule_id)
);

CREATE TABLE business_processes (
    process_id TEXT NOT NULL,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    PRIMARY KEY (document_name, process_id)
);

CREATE TABLE process_stages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL,
    process_id TEXT NOT NULL,
    stage INTEGER NOT NULL,
    stage_name TEXT,
    entry_condition TEXT,
    actions_json TEXT,
    exit_condition TEXT,
    FOREIGN KEY (document_name, process_id) REFERENCES business_processes(document_name, process_id) ON DELETE CASCADE
);

CREATE TABLE exception_handlers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL,
    process_id TEXT NOT NULL,
    scenario TEXT NOT NULL,
    action TEXT NOT NULL,
    FOREIGN KEY (document_name, process_id) REFERENCES business_processes(document_name, process_id) ON DELETE CASCADE
);

-- ============================================
-- 3. 行为层扩展 (Behavior: Flows & State Machines)
-- ============================================

CREATE TABLE flow_specs (
    flow_id TEXT NOT NULL,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    trigger TEXT,
    PRIMARY KEY (document_name, flow_id)
);

CREATE TABLE flow_participants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL,
    flow_id TEXT NOT NULL,
    participant TEXT NOT NULL,
    role TEXT,
    FOREIGN KEY (document_name, flow_id) REFERENCES flow_specs(document_name, flow_id) ON DELETE CASCADE
);

CREATE TABLE flow_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL,
    flow_id TEXT NOT NULL,
    step_order INTEGER NOT NULL,
    actor TEXT,
    action TEXT NOT NULL,
    description TEXT,
    input_json TEXT,
    output_json TEXT,
    error_handling TEXT,
    FOREIGN KEY (document_name, flow_id) REFERENCES flow_specs(document_name, flow_id) ON DELETE CASCADE
);

CREATE TABLE flow_error_handlers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL,
    flow_id TEXT NOT NULL,
    error_type TEXT NOT NULL,
    handler_action TEXT NOT NULL,
    compensation TEXT,
    FOREIGN KEY (document_name, flow_id) REFERENCES flow_specs(document_name, flow_id) ON DELETE CASCADE
);

CREATE TABLE state_machines (
    machine_id TEXT NOT NULL,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    name TEXT NOT NULL,
    initial_state TEXT NOT NULL,
    description TEXT,
    PRIMARY KEY (document_name, machine_id)
);

CREATE TABLE state_transitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL,
    machine_id TEXT NOT NULL,
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    event TEXT,
    guard_condition TEXT,
    action TEXT,
    FOREIGN KEY (document_name, machine_id) REFERENCES state_machines(document_name, machine_id) ON DELETE CASCADE
);

-- ============================================
-- 4. UI 扩展 (Navigation & Responsive)
-- ============================================

CREATE TABLE ui_navigation (
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    navigation_type TEXT,
    PRIMARY KEY (document_name)
);

CREATE TABLE ui_nav_nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    node_id TEXT NOT NULL,
    label TEXT NOT NULL,
    target TEXT,
    icon TEXT,
    children_json TEXT
);

CREATE TABLE ui_nav_transitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    from_screen TEXT NOT NULL,
    to_screen TEXT NOT NULL,
    transition_type TEXT,
    animation TEXT
);

CREATE TABLE responsive_breakpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    breakpoint_name TEXT NOT NULL,
    min_width INTEGER,
    max_width INTEGER,
    layout TEXT
);

CREATE INDEX idx_responsive_breakpoints_name ON responsive_breakpoints(breakpoint_name);

-- ============================================
-- 5. 部署层 (Deployment)
-- ============================================

CREATE TABLE deployment_configs (
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    deployment_type TEXT,
    provider TEXT,
    region TEXT,
    PRIMARY KEY (document_name)
);

CREATE TABLE runtime_requirements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    requirement_type TEXT NOT NULL,
    requirement_value TEXT NOT NULL
);

CREATE TABLE config_vars (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    var_name TEXT NOT NULL,
    var_value TEXT NOT NULL,
    is_secret BOOLEAN NOT NULL DEFAULT FALSE,
    description TEXT
);

CREATE TABLE scaling_strategies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    strategy_type TEXT NOT NULL,
    min_instances INTEGER DEFAULT 1,
    max_instances INTEGER DEFAULT 10,
    target_cpu REAL,
    target_memory REAL
);

-- ============================================
-- 6. 依赖管理 (Dependencies)
-- ============================================

CREATE TABLE dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    dep_name TEXT NOT NULL,
    dep_version TEXT,
    dep_type TEXT NOT NULL DEFAULT 'runtime'
        CHECK (dep_type IN ('runtime', 'build', 'dev', 'optional')),
    source_url TEXT,
    description TEXT
);

CREATE TABLE resources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_name TEXT NOT NULL REFERENCES sdef_documents(name) ON DELETE CASCADE,
    resource_type TEXT NOT NULL,
    resource_name TEXT NOT NULL,
    specification TEXT,
    estimated_cost TEXT
);

-- ============================================
-- 7. 索引
-- ============================================

CREATE INDEX idx_arch_layers_doc ON architecture_layers(document_name);
CREATE INDEX idx_arch_modules_doc ON architecture_modules(document_name);
CREATE INDEX idx_business_concepts_doc ON business_concepts(document_name);
CREATE INDEX idx_flow_specs_doc ON flow_specs(document_name);
CREATE INDEX idx_state_machines_doc ON state_machines(document_name);
CREATE INDEX idx_dependencies_doc ON dependencies(document_name, dep_type);
CREATE INDEX idx_config_vars_doc ON config_vars(document_name);
