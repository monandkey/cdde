-- Create dictionaries table
CREATE TABLE IF NOT EXISTS dictionaries (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    version VARCHAR(50) NOT NULL,
    xml_content TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create dictionary_avps table
CREATE TABLE IF NOT EXISTS dictionary_avps (
    id SERIAL PRIMARY KEY,
    dictionary_id INTEGER REFERENCES dictionaries(id) ON DELETE CASCADE,
    code INTEGER NOT NULL,
    name VARCHAR(255) NOT NULL,
    data_type VARCHAR(50) NOT NULL,
    vendor_id INTEGER,
    UNIQUE(dictionary_id, code)
);

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_dictionary_avps_code ON dictionary_avps(code);
CREATE INDEX IF NOT EXISTS idx_dictionary_avps_dict_id ON dictionary_avps(dictionary_id);
