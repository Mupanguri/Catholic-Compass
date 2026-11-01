# Zimbabwe Catholic Infrastructure Directory
## Project Requirements Document (PRD) — Build Brief
### Version 1.0 | October 2026

---

## 1. Project Overview

A cross-platform mapping and directory application for Roman Catholic infrastructure across Zimbabwe. The application tracks, maps, and provides directions for parishes, outstations (zviteshi / prayer centres), schools, missions, orphanages/children's homes, shrines, and administrative offices (dioceses and deaneries).

The long-term goal is a high-performance backend in Rust with PostgreSQL/PostGIS, paired with a web frontend that can later be ported to mobile (Android/iOS) via Tauri v2 or Dioxus.

**Primary audience:** Catholic faithful in Zimbabwe — urban and rural — seeking to locate, learn about, and connect with Catholic institutions near them. Secondary audience: diocesan administrators and parish priests managing directory accuracy.

---

## 2. Technical Stack

| Layer | Technology |
|---|---|
| Backend / API | Rust (`Axum`) |
| Database | PostgreSQL 16 with `PostGIS` extension |
| ORM / Query | `SQLx` (compile-time checked, not Diesel) |
| Frontend Map | `MapLibre GL JS` (preferred) or `Leaflet.js` |
| Media Storage | AWS S3 (or compatible object store) for institution images |
| Cross-platform | WebAssembly (WASM) first; Tauri v2 / Dioxus mobile later |
| Auth (admin) | JWT-based, minimal — admin panel only |

---

## 3. Database Schema (PostGIS)

All spatial columns use `GEOGRAPHY(Point, 4326)` (lat/lng, WGS84). Apply a GiST index on every `location` column. Apply `pg_trgm` GIN index on `name` columns for fuzzy search.

### 3.1 `dioceses`

```sql
CREATE TABLE dioceses (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(150) NOT NULL,         -- e.g. "Archdiocese of Harare"
    bishop_name VARCHAR(150),
    website     VARCHAR(255),
    created_at  TIMESTAMP DEFAULT NOW()
);
```

### 3.2 `deaneries`

```sql
CREATE TABLE deaneries (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    diocese_id  UUID NOT NULL REFERENCES dioceses(id),
    name        VARCHAR(150) NOT NULL,
    created_at  TIMESTAMP DEFAULT NOW()
);
```

### 3.3 `institution_category` (ENUM)

```sql
CREATE TYPE institution_category AS ENUM (
    'parish',
    'outstation',
    'mission',
    'school',
    'orphanage',
    'shrine',
    'office'
);
```

> **Note on `mission`:** In Zimbabwe Catholic context, missions are distinct rural hubs with their own schools, clinics, and outstations under one administrative centre. They must not be collapsed into `parish` or `outstation`. Keep as a first-class category.

### 3.4 `school_level` (ENUM)

```sql
CREATE TYPE school_level AS ENUM (
    'ecd',
    'primary',
    'secondary',
    'tertiary',
    'vocational'
);
```

### 3.5 `institutions` (master table)

```sql
CREATE TABLE institutions (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name              VARCHAR(255) NOT NULL,
    category          institution_category NOT NULL,
    school_level      school_level,                    -- nullable; only populated when category = 'school'
    diocese_id        UUID REFERENCES dioceses(id),
    deanery_id        UUID REFERENCES deaneries(id),   -- nullable
    parent_id         UUID REFERENCES institutions(id), -- nullable; for outstations → parent parish/mission
    location          GEOGRAPHY(Point, 4326),
    address_notes     TEXT,   -- localized written directions ("turn left 2km past the growth point...")
    contact_phone     VARCHAR(30),
    contact_email     VARCHAR(150),
    website           VARCHAR(255),
    priest_in_charge  VARCHAR(150),
    cover_image_url   VARCHAR(512),                    -- S3 URL for the institution's primary display image
    verified          BOOLEAN NOT NULL DEFAULT FALSE,  -- ground-truth verified flag; all AI-seeded records start FALSE
    created_at        TIMESTAMP DEFAULT NOW(),
    updated_at        TIMESTAMP DEFAULT NOW()
);

-- Spatial index (essential for ST_DWithin performance)
CREATE INDEX idx_institutions_location ON institutions USING GIST (location);

-- Full-text fuzzy search index
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX idx_institutions_name_trgm ON institutions USING GIN (name gin_trgm_ops);
```

### 3.6 `institution_images` (gallery table)

Each institution can have multiple images forming a gallery. One image is designated `is_cover = true` and its URL is also mirrored in `institutions.cover_image_url` for fast retrieval without a join.

```sql
CREATE TABLE institution_images (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    institution_id  UUID NOT NULL REFERENCES institutions(id) ON DELETE CASCADE,
    s3_key          VARCHAR(512) NOT NULL,    -- S3 object key
    url             VARCHAR(512) NOT NULL,    -- full public URL
    caption         VARCHAR(255),
    submitted_by    VARCHAR(150),             -- parishioner name (optional, from submission)
    is_cover        BOOLEAN NOT NULL DEFAULT FALSE,
    approved        BOOLEAN NOT NULL DEFAULT FALSE,  -- admin must approve before public display
    uploaded_at     TIMESTAMP DEFAULT NOW()
);

-- Only one cover image per institution
CREATE UNIQUE INDEX idx_one_cover_per_institution
    ON institution_images (institution_id)
    WHERE is_cover = TRUE AND approved = TRUE;
```

### 3.7 `community_submissions` (crowdsource inbox)

This table captures two types of community input:
1. **"Add Me" submissions** — a parishioner submitting their parish, outstation, or institution that is not yet in the directory.
2. **Image submissions** — a parishioner submitting photos of an existing institution for gallery review.

```sql
CREATE TYPE submission_type AS ENUM (
    'new_institution',   -- "Add Me" — institution not in directory yet
    'image_upload',      -- photo submission for existing institution gallery
    'correction'         -- factual correction to existing record
);

CREATE TYPE submission_status AS ENUM (
    'pending',
    'approved',
    'rejected'
);

CREATE TABLE community_submissions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    submission_type     submission_type NOT NULL,
    status              submission_status NOT NULL DEFAULT 'pending',

    -- Submitter details (all optional — privacy-respecting)
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
    institution_id      UUID REFERENCES institutions(id),  -- which institution the image is for
    image_s3_key        VARCHAR(512),
    image_url           VARCHAR(512),
    image_caption       VARCHAR(255),

    -- For correction submissions
    correction_field    VARCHAR(100),   -- which field is being corrected
    correction_value    TEXT,           -- new value

    -- Admin workflow
    reviewed_by         VARCHAR(150),
    reviewed_at         TIMESTAMP,
    admin_notes         TEXT,

    submitted_at        TIMESTAMP DEFAULT NOW()
);
```

---

## 4. Seed Data

**242 records** are pre-prepared in CSV format covering all 8 Zimbabwean dioceses. The CSV columns match: `name, category, diocese, deanery, town_or_area, lat, lng, notes`.

### Seeding rules:
- All seeded records must be inserted with `verified = FALSE`
- 23 records (all outstations) have no coordinates — insert with `location = NULL`
- The `Mission` category in the CSV maps directly to the `mission` enum value
- `notes` from the CSV maps to `address_notes` in the schema
- One record (`Diocesan OVC / Child Protection Programs`, Diocese of Gokwe) is a program description, not a physical location — skip it during seeding or insert with a comment in `address_notes`

### Dioceses in seed data:
1. Archdiocese of Harare (56 records)
2. Archdiocese of Bulawayo (50 records)
3. Diocese of Gweru (28 records)
4. Diocese of Masvingo (26 records)
5. Diocese of Mutare (25 records)
6. Diocese of Hwange (22 records)
7. Diocese of Gokwe (18 records)
8. Diocese of Chinhoyi (17 records)

---

## 5. API Endpoints

### 5.1 Public endpoints

```
GET  /api/institutions
     Query params: category, diocese_id, deanery_id, verified, search (fuzzy name)
     Returns: paginated list of institutions with cover_image_url

GET  /api/institutions/:id
     Returns: full institution record including gallery images (approved only)

GET  /api/institutions/nearby
     Query params: lat, lng, radius_km (default 10), category
     Uses PostGIS ST_DWithin on GEOGRAPHY column
     Returns: institutions within radius, ordered by distance

GET  /api/dioceses
     Returns: all dioceses with institution counts by category

GET  /api/dioceses/:id/institutions
     Returns: all institutions in diocese, grouped by category

GET  /api/search
     Query params: q (search term)
     Uses pg_trgm similarity on name
     Returns: ranked fuzzy search results across all institutions
```

### 5.2 Submission endpoints (public, rate-limited)

```
POST /api/submissions/institution
     Body: new institution proposal ("Add Me" form)
     Saves to community_submissions with type = 'new_institution'
     Sends notification to admin

POST /api/submissions/image/:institution_id
     Body: multipart form with image file + caption + submitter details
     Uploads image to S3, saves record to community_submissions
     Image is NOT public until admin approves
```

### 5.3 Admin endpoints (JWT-protected)

```
GET  /api/admin/submissions
     Query params: status (pending/approved/rejected), type
     Returns: all community submissions for review

PUT  /api/admin/submissions/:id/approve
     Approves a submission:
     - For new_institution: creates institution record, verified = TRUE
     - For image_upload: creates institution_images record with approved = TRUE,
       updates cover_image_url on institution if is_cover = TRUE

PUT  /api/admin/submissions/:id/reject
     Body: admin_notes (reason)
     Sets status = 'rejected'

PUT  /api/admin/institutions/:id
     Update institution details, toggle verified flag

POST /api/admin/institutions/:id/images
     Direct admin image upload (bypasses submission queue)
     Sets approved = TRUE immediately

DELETE /api/admin/institutions/:id/images/:image_id
     Remove an image from gallery
```

---

## 6. Frontend — Layout & UX Specification

### 6.1 Primary layout: Split view

- **Desktop:** Left sidebar (~360px, fixed) + right map panel (remaining width, full height)
- **Mobile:** Full-screen map + bottom drawer (slides up on marker tap)
- Grid: `grid-template-columns: minmax(320px, 1fr) 2fr`

### 6.2 Sidebar contents (top to bottom)

1. App logo / name
2. Search bar (fuzzy, hits `/api/search`)
3. Category filter pills: All · Parish · Mission · Outstation · School · Shrine · Orphanage
4. Diocese dropdown filter
5. Scrollable institution cards (name, category badge, town, distance if geolocation available)
6. "Add a place" CTA button at bottom

### 6.3 Institution detail view

When a user clicks a map marker or sidebar card, a detail panel opens (desktop: replaces sidebar; mobile: bottom sheet expands). It must contain:

**Header section:**
- Institution name (large)
- Category badge + diocese label
- Verified indicator (subtle green tick if `verified = TRUE`)
- Priest in charge (if available)

**Image gallery section (mandatory):**
- Full-width hero image using `cover_image_url` (fallback: category-appropriate placeholder SVG)
- Below hero: horizontal scrollable thumbnail strip of all approved gallery images
- Each thumbnail is tappable/clickable to expand to full view
- If no images exist: placeholder with text "No photos yet — be the first to submit one"
- "Submit a photo" button always visible below gallery → opens image submission form

**Info section:**
- Contact phone (tap-to-call on mobile)
- Contact email
- Website link
- Address notes (the localized written directions field)
- GPS coordinates (shown as decimal, tappable to open in Google Maps / Apple Maps)

**Action section:**
- "Get Directions" button → opens native maps app with coordinates
- "Submit a correction" link → opens correction form

### 6.4 "Add Me" / community submission form

Accessible via the "Add a place" button in the sidebar. Collects:
- Institution name (required)
- Category (dropdown, required)
- Diocese (dropdown, required)
- Deanery (optional)
- Town / area (required)
- GPS coordinates (optional — with "Use my location" button to auto-fill)
- Written directions / address notes (optional but encouraged)
- Priest in charge (optional)
- Contact phone (optional)
- Submitter name (optional)
- Submitter email (optional — for follow-up)
- Additional notes (optional)

On submit: POST to `/api/submissions/institution`. Show confirmation: "Thank you — our team will review and add your submission."

### 6.5 Image submission form

Accessible from the institution detail gallery section. Collects:
- Image file upload (JPG/PNG, max 10MB)
- Caption (optional)
- Submitter name (optional)
- Submitter email (optional)

On submit: POST to `/api/submissions/image/:institution_id`. Show confirmation: "Thank you — your photo will appear after review."

### 6.6 Map markers

Use custom SVG icon sprites per category. Suggested color coding:
- Parish → deep green
- Mission → forest green (darker)
- Outstation → teal
- School → amber/gold
- Shrine → deep purple
- Orphanage → warm coral
- Office → grey

Cluster markers when zoomed out (MapLibre GL has built-in clustering support).

### 6.7 Visual identity

- **Primary color:** Deep forest green (`#1B4332`) — Marian, Zimbabwean, grounded
- **Accent color:** Warm gold/ochre (`#C9A84C`) — candlelight, the Zimbabwean sun
- **Surface:** Off-white linen (`#F7F5F0`)
- **Typography:** `Plus Jakarta Sans` (humanist sans, Google Fonts) for UI; institution names rendered slightly larger/heavier
- **Category badges:** Pill-shaped, color per category, sentence case labels
- Do NOT use generic blue/grey SaaS palette — this should feel distinctly Zimbabwean and Catholic

---

## 7. Image Gallery — Technical Detail

### Upload flow
1. Client sends multipart POST to `/api/submissions/image/:institution_id`
2. Backend validates file type (JPEG/PNG only) and size (≤ 10MB)
3. Backend generates a UUID-based S3 key: `institutions/{institution_id}/gallery/{uuid}.jpg`
4. Image uploaded to S3 with private ACL initially
5. Record inserted into `community_submissions` with `status = 'pending'`
6. Admin reviews in admin panel

### Approval flow
1. Admin views submission in `/api/admin/submissions`
2. Admin approves → backend:
   - Changes S3 object ACL to public-read
   - Inserts record into `institution_images` with `approved = TRUE`
   - If first approved image for institution: sets `is_cover = TRUE`, updates `institutions.cover_image_url`
3. Image now visible in public gallery

### Cover image logic
- First approved image for an institution automatically becomes cover
- Admin can later designate any approved image as cover via admin endpoint
- Changing cover: set new image `is_cover = TRUE`, unset old cover, update `institutions.cover_image_url`

### Placeholder images
- When `cover_image_url` is NULL, frontend renders a category-specific SVG placeholder
- Placeholders should visually match the app's color identity (green/gold palette)
- Placeholder for parish: simple cross silhouette
- Placeholder for school: open book silhouette
- Placeholder for shrine: candle/flame silhouette
- etc.

---

## 8. Development Phases

### Phase 1 — Database & Backend scaffold
- Rust Cargo project with Axum + SQLx + Tokio
- PostgreSQL + PostGIS Docker Compose setup for local dev
- SQLx migration files for all tables above
- Environment config via `.env` (DATABASE_URL, S3 credentials, JWT secret)

### Phase 2 — Seed data ingestion
- Rust CLI seeder that reads the 8 CSV files
- Maps CSV `category` values to `institution_category` enum
- Looks up / creates diocese and deanery records by name
- Inserts institutions with `verified = FALSE`
- Skips records with obvious data quality issues (flagged in notes above)
- Logs skipped/inserted counts per file

### Phase 3 — Core API endpoints
- All public GET endpoints with filtering, pagination, fuzzy search
- `ST_DWithin` nearby query (GEOGRAPHY column, metres not degrees)
- Submission POST endpoints with S3 upload for images
- Input validation on all endpoints (reject malformed coordinates, oversized files)
- Rate limiting on submission endpoints

### Phase 4 — Frontend map interface
- MapLibre GL JS map, centred on Zimbabwe (`[-19.0, 29.8]`, zoom 6)
- Split-view layout (sidebar + map)
- Category filter pills wired to API
- Institution detail panel with full gallery viewer
- "Add Me" submission form
- Image submission form from detail panel
- Responsive: collapses to full-map + bottom drawer on mobile

### Phase 5 — Admin panel
- Simple protected web UI (can be a separate route `/admin`)
- Submission inbox: list pending submissions, approve/reject with notes
- Image review: preview uploaded image, approve to publish or reject
- Institution editor: edit any field, toggle `verified`, manage gallery order

### Phase 6 — Mobile porting prep
- Abstract API client calls into a shared module
- Ensure all state is driven through the API (no client-side state that can't be recreated)
- Document Tauri v2 / Dioxus wrapping points for future native mobile build

---

## 9. Key Business Rules

1. **All AI-seeded data starts `verified = FALSE`.** The `verified` flag is only set to `TRUE` by an admin, either through the admin panel or when approving a community submission that confirms the record.

2. **No image is ever public until admin-approved.** S3 objects remain private until the approval flow explicitly changes their ACL.

3. **`Mission` is a first-class category**, not an alias for parish or outstation. Do not collapse it.

4. **Outstations should reference their parent institution** via `parent_id`. Where the CSV's notes field names a parent parish or mission, populate `parent_id` during seeding.

5. **The `address_notes` field is a first-class field**, not a catchall. Many Zimbabwean rural locations have no street address — the written directions ("turn left at the borehole, 2km past the growth point") are the primary navigation aid for many users.

6. **Fuzzy search must work in both English and Shona/Ndebele names.** Use `pg_trgm` similarity — do not rely on exact string matching.

7. **Coordinates are approximate for AI-seeded data.** The `verified` flag signals coordinate confidence. The nearby search should note to the user when showing unverified results.

8. **Rate limit community submission endpoints** to prevent spam. Suggested: 5 submissions per IP per hour.

---

## 10. Data Notes for the Build Team

- The CSV seed data uses `Mission` (capital M) as a category — map to `mission` enum
- One record must be skipped: `Diocesan OVC / Child Protection Programs` (Diocese of Gokwe) — it is a program, not a physical location
- 23 outstation records have no lat/lng — insert with `location = NULL`, these will be crowd-filled
- Several Harare outstations note their parent parish in the `notes` field — use this to populate `parent_id`
- `Bondolfi Teachers' College` is the only Catholic teachers' college in Zimbabwe — notable for the UI
- `Mutemwa Leprosy Catholic & Care Centre` (Mutoko) is a major national pilgrimage site — associated with Blessed John Bradburne
- `St Francis Xavier Mission (Empandeni)` is the first Catholic mission established in Zimbabwe (1887) — notable for the UI
- `Dominican Convent High School` (Bulawayo) was founded in 1895 — notable history

---

*End of PRD v1.0*