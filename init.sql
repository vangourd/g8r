-- G8R State Management Database Schema


CREATE TABLE IF NOT EXISTS rosters (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    roster_type VARCHAR(100) NOT NULL,
    traits JSONB NOT NULL DEFAULT '[]'::jsonb,
    connection JSONB NOT NULL DEFAULT '{}'::jsonb,
    auth JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS duties (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    duty_type VARCHAR(100) NOT NULL,
    backend VARCHAR(100) NOT NULL,
    roster_selector JSONB NOT NULL DEFAULT '{}'::jsonb,
    spec JSONB NOT NULL DEFAULT '{}'::jsonb,
    status JSONB DEFAULT '{"phase": "pending"}'::jsonb,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS duty_executions (
    id SERIAL PRIMARY KEY,
    duty_id INTEGER NOT NULL REFERENCES duties(id) ON DELETE CASCADE,
    roster_id INTEGER REFERENCES rosters(id) ON DELETE SET NULL,
    status VARCHAR(50) NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    result JSONB
);

CREATE INDEX idx_rosters_name ON rosters(name);
CREATE INDEX idx_rosters_traits ON rosters USING GIN (traits);
CREATE INDEX idx_duties_name ON duties(name);
CREATE INDEX idx_duties_type ON duties(duty_type);
CREATE INDEX idx_duties_status ON duties USING GIN (status);
CREATE INDEX idx_duty_executions_duty_id ON duty_executions(duty_id);
CREATE INDEX idx_duty_executions_status ON duty_executions(status);

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';


CREATE TRIGGER update_rosters_updated_at BEFORE UPDATE ON rosters
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_duties_updated_at BEFORE UPDATE ON duties
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE IF NOT EXISTS secrets (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    value TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_secrets_name ON secrets(name);

CREATE TRIGGER update_secrets_updated_at BEFORE UPDATE ON secrets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- Stacks: Pull-based reconciliation sources
CREATE TABLE IF NOT EXISTS stacks (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    source_type VARCHAR(50) NOT NULL,
    source_config JSONB NOT NULL,
    config_path VARCHAR(500) NOT NULL,
    reconcile_interval INT,
    last_sync_at TIMESTAMPTZ,
    last_sync_version VARCHAR(255),
    status VARCHAR(50) DEFAULT 'pending',
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_stacks_name ON stacks(name);
CREATE INDEX idx_stacks_status ON stacks(status);
CREATE INDEX idx_stacks_source_type ON stacks(source_type);

CREATE TRIGGER update_stacks_updated_at BEFORE UPDATE ON stacks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- Queues: Push-based pub/sub consumers
CREATE TABLE IF NOT EXISTS queues (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    queue_type VARCHAR(50) NOT NULL,
    queue_config JSONB NOT NULL,
    message_handler VARCHAR(100) NOT NULL,
    handler_config JSONB DEFAULT '{}'::jsonb,
    status VARCHAR(50) DEFAULT 'active',
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_queues_name ON queues(name);
CREATE INDEX idx_queues_status ON queues(status);
CREATE INDEX idx_queues_type ON queues(queue_type);

CREATE TRIGGER update_queues_updated_at BEFORE UPDATE ON queues
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();


-- Unified reconciliation log for stacks and queues
CREATE TABLE IF NOT EXISTS reconciliations (
    id SERIAL PRIMARY KEY,
    source_type VARCHAR(50) NOT NULL,
    source_id INT NOT NULL,
    source_version VARCHAR(255),
    trigger VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    duties_applied JSONB,
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_reconciliations_source ON reconciliations(source_type, source_id);
CREATE INDEX idx_reconciliations_status ON reconciliations(status);
CREATE INDEX idx_reconciliations_started_at ON reconciliations(started_at DESC);
