# Backend README

## Setup

1. Install Rust
2. Install PostgreSQL with PostGIS
3. Copy `.env.example` to `.env` and configure `DATABASE_URL`, `JWT_SECRET`,
   `ADMIN_USERNAME`, and a strong `ADMIN_PASSWORD`
4. Run migrations with sqlx-cli or directly
5. Run `cargo run`

## Migrations
\\\ash
sqlx migrate run
\\\

## Admin moderation

The protected moderation dashboard is available at `/admin` in the frontend.
The backend exposes a login endpoint and requires its JWT for all other
`/api/admin/*` endpoints. Parishioner institution submissions and corrections
remain pending until an authenticated admin approves them.

Image uploads are intentionally disabled during testing. The image submission
endpoints return `501 Not Implemented` until local or object storage is added.

## Seeding
Data is in \seeds/\ directory. Use the seeder CLI (to be implemented).