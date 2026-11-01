-- Dioceses
CREATE TABLE dioceses (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(150) NOT NULL,
    bishop_name VARCHAR(150),
    website     VARCHAR(255),
    created_at  TIMESTAMP DEFAULT NOW()
);

-- Deaneries
CREATE TABLE deaneries (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    diocese_id  UUID NOT NULL REFERENCES dioceses(id),
    name        VARCHAR(150) NOT NULL,
    created_at  TIMESTAMP DEFAULT NOW()
);