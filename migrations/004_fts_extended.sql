-- ============================================
-- Cleanroom Agent - Extended FTS5 Triggers
-- Version: 004
-- Extends FTS5 sync triggers to cover all
-- S.DEF storage tables for full-text search.
-- ============================================

-- Extend the existing sdef_fts table with data from other tables.
-- SQLite FTS5 content tables can be populated via triggers on content tables.
-- Since sdef_fts uses sdef_documents as content table (for the rowid),
-- we use external content FTS queries by inserting into sdef_fts directly.

-- ============================================
-- 1. FTS triggers for data_models
-- ============================================
CREATE TRIGGER IF NOT EXISTS data_models_fts_insert
AFTER INSERT ON data_models
BEGIN
    INSERT INTO sdef_fts(rowid, document_name, entity_name, description)
    SELECT rowid, NEW.document_name, NEW.entity, COALESCE(NEW.description, '')
    FROM sdef_documents WHERE name = NEW.document_name;
END;

CREATE TRIGGER IF NOT EXISTS data_models_fts_delete
AFTER DELETE ON data_models
BEGIN
    INSERT INTO sdef_fts(sdef_fts, rowid, document_name, entity_name, description)
    VALUES ('delete', (
        SELECT rowid FROM sdef_documents WHERE name = OLD.document_name
    ), OLD.document_name, OLD.entity, OLD.description);
END;

-- ============================================
-- 2. FTS triggers for contracts
-- ============================================
CREATE TRIGGER IF NOT EXISTS contracts_fts_insert
AFTER INSERT ON contracts
BEGIN
    INSERT INTO sdef_fts(rowid, document_name, entity_name, description)
    SELECT rowid, NEW.document_name, NEW.name, COALESCE(NEW.description, '')
    FROM sdef_documents WHERE name = NEW.document_name;
END;

CREATE TRIGGER IF NOT EXISTS contracts_fts_delete
AFTER DELETE ON contracts
BEGIN
    INSERT INTO sdef_fts(sdef_fts, rowid, document_name, entity_name, description)
    VALUES ('delete', (
        SELECT rowid FROM sdef_documents WHERE name = OLD.document_name
    ), OLD.document_name, OLD.name, OLD.description);
END;

-- ============================================
-- 3. FTS triggers for function_specs
-- ============================================
CREATE TRIGGER IF NOT EXISTS function_specs_fts_insert
AFTER INSERT ON function_specs
BEGIN
    INSERT INTO sdef_fts(rowid, document_name, entity_name, description)
    SELECT rowid, NEW.document_name, NEW.name, COALESCE(NEW.description, '')
    FROM sdef_documents WHERE name = NEW.document_name;
END;

CREATE TRIGGER IF NOT EXISTS function_specs_fts_delete
AFTER DELETE ON function_specs
BEGIN
    INSERT INTO sdef_fts(sdef_fts, rowid, document_name, entity_name, description)
    VALUES ('delete', (
        SELECT rowid FROM sdef_documents WHERE name = OLD.document_name
    ), OLD.document_name, OLD.name, OLD.description);
END;

-- ============================================
-- 4. FTS triggers for business_concepts
-- ============================================
CREATE TRIGGER IF NOT EXISTS business_concepts_fts_insert
AFTER INSERT ON business_concepts
BEGIN
    INSERT INTO sdef_fts(rowid, document_name, entity_name, description)
    SELECT rowid, NEW.document_name, NEW.name, COALESCE(NEW.description, '')
    FROM sdef_documents WHERE name = NEW.document_name;
END;

CREATE TRIGGER IF NOT EXISTS business_concepts_fts_delete
AFTER DELETE ON business_concepts
BEGIN
    INSERT INTO sdef_fts(sdef_fts, rowid, document_name, entity_name, description)
    VALUES ('delete', (
        SELECT rowid FROM sdef_documents WHERE name = OLD.document_name
    ), OLD.document_name, OLD.name, OLD.description);
END;

-- ============================================
-- 5. FTS triggers for architecture_modules
-- ============================================
CREATE TRIGGER IF NOT EXISTS architecture_modules_fts_insert
AFTER INSERT ON architecture_modules
BEGIN
    INSERT INTO sdef_fts(rowid, document_name, entity_name, description)
    SELECT rowid, NEW.document_name, NEW.module_name, COALESCE(NEW.responsibility, '')
    FROM sdef_documents WHERE name = NEW.document_name;
END;

CREATE TRIGGER IF NOT EXISTS architecture_modules_fts_delete
AFTER DELETE ON architecture_modules
BEGIN
    INSERT INTO sdef_fts(sdef_fts, rowid, document_name, entity_name, description)
    VALUES ('delete', (
        SELECT rowid FROM sdef_documents WHERE name = OLD.document_name
    ), OLD.document_name, OLD.module_name, OLD.responsibility);
END;

-- ============================================
-- 6. FTS triggers for flow_specs
-- ============================================
CREATE TRIGGER IF NOT EXISTS flow_specs_fts_insert
AFTER INSERT ON flow_specs
BEGIN
    INSERT INTO sdef_fts(rowid, document_name, entity_name, description)
    SELECT rowid, NEW.document_name, NEW.name, COALESCE(NEW.description, '')
    FROM sdef_documents WHERE name = NEW.document_name;
END;

CREATE TRIGGER IF NOT EXISTS flow_specs_fts_delete
AFTER DELETE ON flow_specs
BEGIN
    INSERT INTO sdef_fts(sdef_fts, rowid, document_name, entity_name, description)
    VALUES ('delete', (
        SELECT rowid FROM sdef_documents WHERE name = OLD.document_name
    ), OLD.document_name, OLD.name, OLD.description);
END;

-- ============================================
-- 7. FTS triggers for design_decisions
-- ============================================
CREATE TRIGGER IF NOT EXISTS design_decisions_fts_insert
AFTER INSERT ON design_decisions
BEGIN
    INSERT INTO sdef_fts(rowid, document_name, entity_name, description)
    SELECT rowid, NEW.document_name, NEW.topic, COALESCE(NEW.decision, '')
    FROM sdef_documents WHERE name = NEW.document_name;
END;

CREATE TRIGGER IF NOT EXISTS design_decisions_fts_delete
AFTER DELETE ON design_decisions
BEGIN
    INSERT INTO sdef_fts(sdef_fts, rowid, document_name, entity_name, description)
    VALUES ('delete', (
        SELECT rowid FROM sdef_documents WHERE name = OLD.document_name
    ), OLD.document_name, OLD.topic, OLD.decision);
END;

-- ============================================
-- 8. FTS triggers for ui_screens
-- ============================================
CREATE TRIGGER IF NOT EXISTS ui_screens_fts_insert
AFTER INSERT ON ui_screens
BEGIN
    INSERT INTO sdef_fts(rowid, document_name, entity_name, description)
    SELECT rowid, NEW.document_name, NEW.name, COALESCE(NEW.purpose, '')
    FROM sdef_documents WHERE name = NEW.document_name;
END;

CREATE TRIGGER IF NOT EXISTS ui_screens_fts_delete
AFTER DELETE ON ui_screens
BEGIN
    INSERT INTO sdef_fts(sdef_fts, rowid, document_name, entity_name, description)
    VALUES ('delete', (
        SELECT rowid FROM sdef_documents WHERE name = OLD.document_name
    ), OLD.document_name, OLD.name, OLD.purpose);
END;

-- ============================================
-- 9. FTS triggers for business_rules
-- ============================================
CREATE TRIGGER IF NOT EXISTS business_rules_fts_insert
AFTER INSERT ON business_rules
BEGIN
    INSERT INTO sdef_fts(rowid, document_name, entity_name, description)
    SELECT rowid, NEW.document_name, NEW.name, COALESCE(NEW.description, '')
    FROM sdef_documents WHERE name = NEW.document_name;
END;

CREATE TRIGGER IF NOT EXISTS business_rules_fts_delete
AFTER DELETE ON business_rules
BEGIN
    INSERT INTO sdef_fts(sdef_fts, rowid, document_name, entity_name, description)
    VALUES ('delete', (
        SELECT rowid FROM sdef_documents WHERE name = OLD.document_name
    ), OLD.document_name, OLD.name, OLD.description);
END;
