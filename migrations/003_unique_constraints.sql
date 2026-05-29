-- Migration 003: Add UNIQUE constraints to prevent duplicate entities

-- function_specs: prevent duplicate function names per document
-- Clean up any existing duplicates first (keep the first entry)
DELETE FROM function_specs WHERE id NOT IN (
    SELECT MIN(id) FROM function_specs GROUP BY document_name, name
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_function_specs_unique
    ON function_specs(document_name, name);

-- data_models already has UNIQUE(document_name, entity) via PRIMARY KEY but ensure index exists
CREATE UNIQUE INDEX IF NOT EXISTS idx_data_models_unique
    ON data_models(document_name, entity);

-- contracts already has PRIMARY KEY(document_name, name), ensure index
CREATE UNIQUE INDEX IF NOT EXISTS idx_contracts_unique
    ON contracts(document_name, name);
