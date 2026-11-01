-- Institution images gallery
CREATE TABLE institution_images (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    institution_id  UUID NOT NULL REFERENCES institutions(id) ON DELETE CASCADE,
    s3_key          VARCHAR(512) NOT NULL,
    url             VARCHAR(512) NOT NULL,
    caption         VARCHAR(255),
    submitted_by    VARCHAR(150),
    is_cover        BOOLEAN NOT NULL DEFAULT FALSE,
    approved        BOOLEAN NOT NULL DEFAULT FALSE,
    uploaded_at     TIMESTAMP DEFAULT NOW()
);

-- Only one cover image per institution
CREATE UNIQUE INDEX idx_one_cover_per_institution
    ON institution_images (institution_id)
    WHERE is_cover = TRUE AND approved = TRUE;