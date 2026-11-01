-- Enable extensions
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS postgis;

-- Enums
CREATE TYPE institution_category AS ENUM (
    'parish',
    'outstation',
    'mission',
    'school',
    'orphanage',
    'shrine',
    'office'
);

CREATE TYPE school_level AS ENUM (
    'ecd',
    'primary',
    'secondary',
    'tertiary',
    'vocational'
);

CREATE TYPE submission_type AS ENUM (
    'new_institution',
    'image_upload',
    'correction'
);

CREATE TYPE submission_status AS ENUM (
    'pending',
    'approved',
    'rejected'
);