# Catholic Compass

## Zimbabwe Catholic Directory

Catholic Compass is a map-based directory for finding Catholic places of
faith across Zimbabwe. It helps parishioners, pilgrims, families, students,
travellers, and clergy find a nearby or regional parish, mission, outstation,
school, shrine, orphanage, or diocesan office.

The project is built around a simple pastoral idea: we are a pilgrim Church.
When one member moves to a new town, city, province, or country, the journey
of faith does not end. With the gifts of God and the guidance of the Holy
Spirit, Catholic Compass acts as a practical compass to help people reconnect
with the Catholic community wherever they are.

The directory is community-informed but administrator-reviewed. Parishioners
can suggest new places and report corrections. Submissions remain pending
until an authenticated administrator reviews and approves them.

## What the application does

### Public experience

- Presents an informational Catholic Compass landing page.
- Explains the purpose and faith story behind the directory.
- Opens an interactive map centered on Zimbabwe.
- Uses the CARTO Voyager vector map style.
- Displays institutions as category-coloured markers.
- Groups nearby markers into clusters.
- Supports searching institutions.
- Filters by institution category and diocese.
- Opens an institution detail panel.
- Flies the map to the selected institution.
- Shows contact, address, priest, website, and verification information.
- Calculates a driving route on the map using OSRM when browser location is
  allowed.
- Displays route distance and estimated driving time.
- Can open Google Maps as an external navigation fallback.
- Allows parishioners to suggest a new institution.
- Allows parishioners to submit a correction for an existing institution.

### Administration experience

The `/admin` page provides:

- Admin login using credentials configured in the backend environment.
- JWT-protected moderation requests.
- A review queue for community submissions.
- Counts for total institutions, pending reviews, and verified institutions.
- Approve actions for new institutions and corrections.
- Reject actions requiring an administrator note.
- Reviewer and review-time tracking in the database.
- A public-map link and sign-out action.
- A liturgical season indicator.

## Technology

### Frontend

- Vanilla JavaScript and HTML.
- CSS with responsive layouts.
- Vite development server and production build.
- MapLibre GL JS.
- CARTO Voyager vector tiles.
- OpenStreetMap-derived mapping data through the selected map provider.

### Backend

- Rust.
- Axum HTTP server.
- Tokio asynchronous runtime.
- SQLx PostgreSQL driver.
- JSON REST API.
- JWT authentication for admin endpoints.
- CORS and tracing middleware.

### Database

- PostgreSQL.
- PostGIS for geographic points and spatial queries.
- `pg_trgm` for fuzzy institution-name searching.
- UUID primary keys.
- PostgreSQL enums for institution categories and submission states.

### Local infrastructure

- Docker Compose for PostgreSQL/PostGIS.
- Rust Cargo for the backend.
- npm for the frontend.

## Repository layout

```text
.
├── backend/
│   ├── migrations/       PostgreSQL and PostGIS schema migrations
│   ├── seeds/            Source seed data
│   ├── seeder/           Seeder binary source
│   ├── src/              Rust API source
│   ├── .env.example      Environment template
│   ├── Cargo.toml        Rust dependencies and binaries
│   └── README.md         Backend-specific notes
├── docker/
│   └── docker-compose.yml PostgreSQL/PostGIS service
├── frontend/
│   ├── public/assets/    Compass branding assets
│   ├── src/main.js       Public map and admin UI
│   ├── src/style.css     Public, modal, map, and admin styling
│   ├── src/utils/api.js  Frontend API client
│   ├── index.html        Browser entry point
│   ├── package.json      Frontend scripts
│   └── README.md         Frontend-specific notes
├── Compass.svg           Original compass logo
├── Compass.jpeg          Original landing-page image
├── instructions.md       Product requirements and design notes
└── README.md             This document
```

## Requirements

Install the following before starting:

- Rust and Cargo.
- Node.js and npm.
- Docker and Docker Compose.
- `sqlx-cli` for applying migrations.

Install SQLx CLI if needed:

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

## First-time setup

Run all commands from the project root:

```bash
cd "/run/media/kali/New Volume/Me/MAPME"
```

### 1. Start PostgreSQL/PostGIS

```bash
sudo docker-compose -f docker/docker-compose.yml up -d postgres
```

Verify the container:

```bash
sudo docker-compose -f docker/docker-compose.yml ps
```

The development database is configured by Docker with:

```text
Database: mapme
User: mapme
Password: mapme
Host: localhost
Port: 5432
```

### 2. Configure the backend

Create the backend environment file:

```bash
cp backend/.env.example backend/.env
```

Set local values in `backend/.env`:

```env
DATABASE_URL=postgres://mapme:mapme@localhost:5432/mapme
JWT_SECRET=replace-with-a-long-random-secret
ADMIN_USERNAME=admin
ADMIN_PASSWORD=replace-with-a-strong-password
RUST_LOG=info
```

Do not commit `.env`, passwords, JWT secrets, or cloud credentials. The
development image-upload integration is currently disabled and no AWS/S3
configuration is required.

### 3. Apply migrations

```bash
cd backend
sqlx migrate run
cd ..
```

The migrations create:

- PostGIS and fuzzy-search extensions.
- Dioceses and deaneries.
- Institutions and spatial indexes.
- Institution image metadata.
- Community submissions and moderation status fields.

### 4. Start the backend

In one terminal:

```bash
cd "/run/media/kali/New Volume/Me/MAPME/backend"
cargo run
```

The API listens on:

```text
http://127.0.0.1:8080
```

Check that it is running:

```bash
curl http://127.0.0.1:8080/health
```

Expected response:

```text
OK
```

Keep this terminal open while using the frontend.

### 5. Start the frontend

In a second terminal:

```bash
cd "/run/media/kali/New Volume/Me/MAPME/frontend"
npm install
npm run dev
```

Open the URL printed by Vite, normally:

```text
http://localhost:5173
```

The admin dashboard is:

```text
http://localhost:5173/admin
```

Use the `ADMIN_USERNAME` and `ADMIN_PASSWORD` values from `backend/.env`.

## Public workflow

### Finding an institution

1. Open the public frontend.
2. Read the Catholic Compass introduction.
3. Select **Take me to the map**.
4. Search by institution name or filter by category/diocese.
5. Select a list card or map marker.
6. The map flies to the institution and opens its detail panel.

### Getting directions

The detail panel provides two navigation choices:

- **Route on Map**:
  - Requests the browser's current location.
  - Calls the public OSRM driving router.
  - Draws the route directly on MapLibre.
  - Fits the map to the route.
  - Displays approximate road distance and travel time.
- **Open Maps**:
  - Uses browser geolocation when available.
  - Opens an external Google Maps route.
  - Falls back to a destination search if location access is denied.

Browser geolocation requires a secure context (HTTPS, or localhost during
development) and user permission. If the site is opened over plain HTTP, the
browser will block location before the routing service is called. In-map
routing currently supports driving only.

### Suggesting a new place

1. Select **Add a place** in the map sidebar.
2. Complete the custom form.
3. Provide a name and category.
4. Add coordinates if known.
5. Add address notes and optional contact information.
6. Submit the form.

The backend validates the category and coordinate ranges, then stores the
submission as `pending`. It does not appear as a published institution until
an administrator approves it.

### Reporting a correction

From an institution detail panel, select **Submit a correction**. The current
flow records the field, proposed value, and optional notes as a correction
submission. Administrators review the correction before it changes the
published institution.

## Administration workflow

1. Open `/admin`.
2. Sign in with the configured admin credentials.
3. Review the pending queue.
4. Inspect the proposed institution or correction.
5. Approve valid submissions.
6. Reject invalid submissions with a reason.
7. Sign out when finished.

Approval is protected by a JWT and performed transactionally for new
institutions and corrections. A correction can update approved fields such as
name, address notes, contact phone, contact email, website, and
priest-in-charge.

## API reference

### Health

```text
GET /health
```

### Public institution endpoints

```text
GET /api/institutions
GET /api/institutions/:id
GET /api/institutions/nearby
GET /api/dioceses
GET /api/dioceses/:id/institutions
GET /api/search?q=...
```

Institution-list filters include:

```text
category
diocese_id
deanery_id
verified
search
page
page_size
```

### Public submissions

```text
POST /api/submissions/institution
POST /api/submissions/correction
```

New-institution submissions use fields including:

```json
{
  "proposed_name": "Example Parish",
  "proposed_category": "parish",
  "proposed_lat": -17.8292,
  "proposed_lng": 31.0522,
  "proposed_address_notes": "Example address",
  "submitter_name": "Optional name",
  "submitter_email": "optional@example.com"
}
```

### Admin authentication

```text
POST /api/admin/login
```

The login response contains a JWT. Protected requests use:

```text
Authorization: Bearer <token>
```

### Protected admin endpoints

```text
GET /api/admin/submissions
PUT /api/admin/submissions/:id/approve
PUT /api/admin/submissions/:id/reject
PUT /api/admin/institutions/:id
```

Image-management routes exist as placeholders but currently return
`501 Not Implemented` because image storage is intentionally disabled during
testing.

## Data model

### Institutions

Institutions include:

- Name.
- Category.
- Optional school level.
- Diocese and deanery relationships.
- Parent institution relationship.
- PostGIS geography point.
- Address notes.
- Phone, email, and website.
- Priest in charge.
- Cover-image URL metadata.
- Verification status.
- Creation and update timestamps.

### Community submissions

Submissions support:

- New institutions.
- Image submissions in the schema for future use.
- Corrections to existing institutions.
- Submitter contact details.
- Proposed coordinates and descriptive fields.
- Review status: pending, approved, or rejected.
- Reviewer identity, review time, and administrator notes.

## Validation and verification

Run the frontend production build:

```bash
cd frontend
npm run build
```

Run the backend compiler/type check:

```bash
cd backend
cargo check
```

Run both before submitting changes. A successful build does not replace
manual browser testing.

Recommended manual checks:

1. Load the landing page.
2. Enter the map.
3. Search and filter institutions.
4. Select a sidebar card and verify the camera centers on it.
5. Open an institution detail panel.
6. Test the route action with location permission allowed and denied.
7. Submit a new place.
8. Submit a correction.
9. Sign in at `/admin`.
10. Approve one pending item.
11. Reject another item with a reason.
12. Confirm the public map reflects approved data.

## Current limitations

The following items are intentionally incomplete or limited during testing:

- Image uploads and AWS/S3 storage are disabled.
- Image endpoints return `501 Not Implemented`.
- The admin login currently uses one environment-configured admin account.
- Full multi-user RBAC with `super_admin` and `category_admin` roles is not yet
  implemented.
- Category-admin assignment and category-scoped permissions are not yet
  available.
- The admin dashboard currently focuses on moderation rather than full
  institution CRUD editing.
- Public correction entry is functional but can be improved with a richer
  form and field-specific validation.
- The public OSRM service is suitable for development and testing, not a
  guaranteed production routing service.
- Configure the frontend API origin with `VITE_API_URL`. For Vercel, set it
  to the deployed Render origin (without `/api`), then redeploy.
- CORS is permissive for local development and should be restricted before
  production deployment.
- The database seeder workflow still needs a finalized production command.

## Security notes

- Never publish `backend/.env`.
- Never share passwords, JWT secrets, database credentials, or API keys.
- Replace development credentials before deployment.
- Use a strong random `JWT_SECRET`.
- Restrict CORS to the deployed frontend origin.
- Put the backend behind HTTPS in production.
- Add rate limiting and abuse protection to public submissions.
- Add stronger administrator identity management before multiple admins are
  deployed.
- Keep all moderation endpoints protected.
- Validate uploaded files before enabling image storage.

## Deployment direction

A production deployment should include:

1. Managed PostgreSQL with PostGIS.
2. HTTPS frontend and backend origins.
3. Secret management outside committed files.
4. Restricted CORS.
5. Persistent logging and monitoring.
6. Backups for database and submission records.
7. Authenticated multi-user administration.
8. Moderation audit logs.
9. A supported tile and routing-provider plan.
10. Local or object storage with malware and file-type validation if images are
    enabled.

## Project story

Catholic Compass began from a grassroots need: people move, communities
change, and faith communities should remain findable. A parishioner arriving
in a new place should not have to begin their search without guidance. A
traveller should be able to find a Mass community. A family should be able to
locate a school, mission, shrine, or outstation. A local community should be
able to help keep the directory accurate.

The application turns that need into a shared, reviewed map. Parishioners
contribute knowledge. Administrators protect accuracy. The map provides
direction. The Catholic Compass is therefore both a practical directory and
an expression of connection: movement does not have to mean separation from
the Church.

---

Akroatis copyright 2026
