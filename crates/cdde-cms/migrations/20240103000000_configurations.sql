-- Create routing_rules table
CREATE TABLE IF NOT EXISTS routing_rules (
    id SERIAL PRIMARY KEY,
    vr_id VARCHAR(255) REFERENCES virtual_routers(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL,
    realm VARCHAR(255),
    application_id INTEGER,
    destination_host VARCHAR(255),
    target_pool VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create manipulation_rules table
CREATE TABLE IF NOT EXISTS manipulation_rules (
    id SERIAL PRIMARY KEY,
    vr_id VARCHAR(255) REFERENCES virtual_routers(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL,
    rule_json JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for faster lookups
CREATE INDEX IF NOT EXISTS idx_routing_rules_vr_id ON routing_rules(vr_id);
CREATE INDEX IF NOT EXISTS idx_routing_rules_priority ON routing_rules(priority);
CREATE INDEX IF NOT EXISTS idx_manipulation_rules_vr_id ON manipulation_rules(vr_id);
CREATE INDEX IF NOT EXISTS idx_manipulation_rules_priority ON manipulation_rules(priority);
