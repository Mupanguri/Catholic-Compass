-- Community submissions
CREATE TABLE community_submissions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    submission_type     submission_type NOT NULL,
    status              submission_status NOT NULL DEFAULT 'pending',

    -- Submitter details
    submitter_name      VARCHAR(150),
    submitter_email     VARCHAR(150),
    submitter_phone     VARCHAR(30),

    -- For new_institution submissions
    proposed_name       VARCHAR(255),
    proposed_category   institution_category,
    proposed_diocese    VARCHAR(150),
    proposed_deanery    VARCHAR(150),
    proposed_lat        DOUBLE PRECISION,
    proposed_lng        DOUBLE PRECISION,
    proposed_address_notes TEXT,
    proposed_phone      VARCHAR(30),
    proposed_priest     VARCHAR(150),
    additional_notes    TEXT,

    -- For image_upload submissions
    institution_id      UUID REFERENCES institutions(id),
    image_s3_key        VARCHAR(512),
    image_url           VARCHAR(512),
    image_caption       VARCHAR(255),

    -- For correction submissions
    correction_field    VARCHAR(100),
    correction_value    TEXT,

    -- Admin workflow
    reviewed_by         VARCHAR(150),
    reviewed_at         TIMESTAMP,
    admin_notes         TEXT,

    submitted_at        TIMESTAMP DEFAULT NOW()
);