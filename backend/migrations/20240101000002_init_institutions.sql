-- Institutions
CREATE TABLE institutions (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name              VARCHAR(255) NOT NULL,
    category          institution_category NOT NULL,
    school_level      school_level,
    diocese_id        UUID REFERENCES dioceses(id),
    deanery_id        UUID REFERENCES deaneries(id),
    parent_id         UUID REFERENCES institutions(id),
    location          GEOGRAPHY(Point, 4326),
    address_notes     TEXT,
    contact_phone     VARCHAR(30),
    contact_email     VARCHAR(150),
    website           VARCHAR(255),
    priest_in_charge  VARCHAR(150),
    cover_image_url   VARCHAR(512),
    verified          BOOLEAN NOT NULL DEFAULT FALSE,
    created_at        TIMESTAMP DEFAULT NOW(),
    updated_at        TIMESTAMP DEFAULT NOW()
);

-- Spatial index
CREATE INDEX idx_institutions_location ON institutions USING GIST (location);

-- Fuzzy search index
CREATE INDEX idx_institutions_name_trgm ON institutions USING GIN (name gin_trgm_ops);