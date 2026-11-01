use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

use crate::db::Pool;
use crate::models::*;

pub type AppState = Arc<AppStateInner>;

pub struct AppStateInner {
    pub pool: Pool,
    pub jwt_secret: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        // Public institution endpoints
        .route("/api/institutions", get(list_institutions))
        .route("/api/institutions/:id", get(get_institution))
        .route("/api/institutions/nearby", get(nearby_institutions))
        // Diocese endpoints
        .route("/api/dioceses", get(list_dioceses))
        .route("/api/dioceses/:id/institutions", get(diocese_institutions))
        // Search
        .route("/api/search", get(search_institutions))
        // Submissions
        .route("/api/admin/login", post(admin_login))
        .route("/api/submissions/institution", post(create_institution_submission))
        .route("/api/submissions/correction", post(create_correction_submission))
        .route(
            "/api/submissions/image/:institution_id",
            post(create_image_submission),
        )
        // Admin endpoints
        .route("/api/admin/submissions", get(list_submissions))
        .route(
            "/api/admin/submissions/:id/approve",
            put(approve_submission),
        )
        .route(
            "/api/admin/submissions/:id/reject",
            put(reject_submission),
        )
        .route("/api/admin/institutions/:id", put(update_institution))
        .route(
            "/api/admin/institutions/:id/images",
            post(admin_upload_image),
        )
        .route(
            "/api/admin/institutions/:id/images/:image_id",
            delete(delete_image),
        )
}

// ---------- Public: list institutions ----------

async fn list_institutions(
    State(state): State<AppState>,
    Query(params): Query<InstitutionListQuery>,
) -> Result<Json<Value>, StatusCode> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;

    let mut query = String::from(
        "SELECT id, name, category::text as category, school_level::text as school_level, diocese_id, deanery_id, parent_id,
                 ST_AsText(location) as location, address_notes, contact_phone, contact_email,
                 website, priest_in_charge, cover_image_url, verified, created_at, updated_at
          FROM institutions",
    );
    let mut conditions: Vec<String> = Vec::new();
    let mut param_count: i32 = 0;

    if let Some(ref cat) = params.category {
        param_count += 1;
        conditions.push(format!("category = ${}::institution_category", param_count));
    }
    if let Some(did) = params.diocese_id {
        param_count += 1;
        conditions.push(format!("diocese_id = ${}", param_count));
    }
    if let Some(did) = params.deanery_id {
        param_count += 1;
        conditions.push(format!("deanery_id = ${}", param_count));
    }
    if let Some(v) = params.verified {
        param_count += 1;
        conditions.push(format!("verified = ${}", param_count));
    }
    if let Some(ref s) = params.search {
        param_count += 1;
        conditions.push(format!("name % ${}", param_count));
    }

    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }

    query.push_str(" ORDER BY name ASC");
    param_count += 1;
    query.push_str(&format!(" LIMIT ${}", param_count));
    let limit_param = param_count;
    param_count += 1;
    query.push_str(&format!(" OFFSET ${}", param_count));
    let offset_param = param_count;

    let mut qb = sqlx::query_as::<_, Institution>(&query);

    let mut idx = 0;
    if let Some(ref cat) = params.category {
        idx += 1;
        qb = qb.bind(cat);
    }
    if let Some(did) = params.diocese_id {
        idx += 1;
        qb = qb.bind(did);
    }
    if let Some(did) = params.deanery_id {
        idx += 1;
        qb = qb.bind(did);
    }
    if let Some(v) = params.verified {
        idx += 1;
        qb = qb.bind(v);
    }
    if let Some(ref s) = params.search {
        idx += 1;
        qb = qb.bind(s);
    }
    qb = qb.bind(page_size as i32).bind(offset as i32);

    let institutions = qb.fetch_all(&state.pool).await
        .map_err(|e| { tracing::error!("list institutions error: {:?}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    // Count total
    let mut count_query = String::from("SELECT COUNT(*) FROM institutions");
    if !conditions.is_empty() {
        count_query.push_str(" WHERE ");
        count_query.push_str(&conditions.join(" AND "));
    }
    let mut cq = sqlx::query(&count_query);
    idx = 0;
    if let Some(ref cat) = params.category {
        idx += 1;
        cq = cq.bind(cat);
    }
    if let Some(did) = params.diocese_id {
        idx += 1;
        cq = cq.bind(did);
    }
    if let Some(did) = params.deanery_id {
        idx += 1;
        cq = cq.bind(did);
    }
    if let Some(v) = params.verified {
        idx += 1;
        cq = cq.bind(v);
    }
    if let Some(ref s) = params.search {
        idx += 1;
        cq = cq.bind(s);
    }
    let row = cq.fetch_one(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let total: i64 = row.try_get::<i64, _>(0).unwrap_or(0);

    Ok(Json(json!(PaginatedInstitutions {
        institutions,
        total,
        page,
        page_size,
    })))
}

// ---------- Public: get institution detail ----------

async fn get_institution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let inst = sqlx::query_as::<_, InstitutionDetail>(
        "SELECT id, name, category::text as category, school_level::text as school_level, diocese_id, deanery_id, parent_id,
                 ST_AsText(location) as location, address_notes, contact_phone, contact_email,
                 website, priest_in_charge, cover_image_url, verified, created_at, updated_at
          FROM institutions WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let images = sqlx::query(
        "SELECT id, s3_key, url, caption, submitted_by, is_cover, approved, uploaded_at
          FROM institution_images WHERE institution_id = $1 AND approved = TRUE
          ORDER BY is_cover DESC, uploaded_at ASC",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let image_list: Vec<InstitutionImage> = images
        .iter()
        .map(|r| InstitutionImage {
            id: r.try_get::<Uuid, _>("id").unwrap_or(Uuid::nil()),
            s3_key: r.try_get::<String, _>("s3_key").unwrap_or_default(),
            url: r.try_get::<String, _>("url").unwrap_or_default(),
            caption: r.try_get::<Option<String>, _>("caption").unwrap_or(None),
            submitted_by: r.try_get::<Option<String>, _>("submitted_by").unwrap_or(None),
            is_cover: r.try_get::<bool, _>("is_cover").unwrap_or(false),
            approved: r.try_get::<bool, _>("approved").unwrap_or(false),
            uploaded_at: r.try_get::<NaiveDateTime, _>("uploaded_at").unwrap_or(Utc::now().naive_utc()),
        })
        .collect();

    Ok(Json(json!({
        "institution": inst,
        "images": image_list
    })))
}

// ---------- Public: nearby institutions ----------

async fn nearby_institutions(
    State(state): State<AppState>,
    Query(params): Query<NearbyQuery>,
) -> Result<Json<Value>, StatusCode> {
    let radius_m = params.radius_km.unwrap_or(10.0) * 1000.0;

    let rows = sqlx::query(
        "SELECT id, name, category::text as category, school_level::text as school_level, diocese_id, deanery_id, parent_id,
                 ST_AsText(location) as location, address_notes, contact_phone, contact_email,
                 website, priest_in_charge, cover_image_url, verified, created_at, updated_at,
                 ST_Distance(location, ST_SetSRID(ST_MakePoint($2, $3), 4326)::geography) as distance_meters
          FROM institutions
          WHERE location IS NOT NULL
            AND ST_DWithin(location, ST_SetSRID(ST_MakePoint($2, $3), 4326)::geography, $1)",
    )
    .bind(radius_m as f64)
    .bind(params.lng)
    .bind(params.lat)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut results: Vec<Value> = Vec::new();
    for r in rows {
        let distance: Option<f64> = r.try_get("distance_meters").ok();
        results.push(json!({
            "id": r.try_get::<Uuid, _>("id").unwrap_or(Uuid::nil()),
            "name": r.try_get::<String, _>("name").unwrap_or_default(),
            "category": r.try_get::<String, _>("category").unwrap_or_default(),
            "school_level": r.try_get::<Option<String>, _>("school_level").unwrap_or(None),
            "diocese_id": r.try_get::<Option<Uuid>, _>("diocese_id").unwrap_or(None),
            "deanery_id": r.try_get::<Option<Uuid>, _>("deanery_id").unwrap_or(None),
            "parent_id": r.try_get::<Option<Uuid>, _>("parent_id").unwrap_or(None),
            "location": r.try_get::<Option<String>, _>("location").unwrap_or(None),
            "address_notes": r.try_get::<Option<String>, _>("address_notes").unwrap_or(None),
            "contact_phone": r.try_get::<Option<String>, _>("contact_phone").unwrap_or(None),
            "contact_email": r.try_get::<Option<String>, _>("contact_email").unwrap_or(None),
            "website": r.try_get::<Option<String>, _>("website").unwrap_or(None),
            "priest_in_charge": r.try_get::<Option<String>, _>("priest_in_charge").unwrap_or(None),
            "cover_image_url": r.try_get::<Option<String>, _>("cover_image_url").unwrap_or(None),
            "verified": r.try_get::<bool, _>("verified").unwrap_or(false),
            "created_at": r.try_get::<NaiveDateTime, _>("created_at").unwrap_or(Utc::now().naive_utc()),
            "updated_at": r.try_get::<NaiveDateTime, _>("updated_at").unwrap_or(Utc::now().naive_utc()),
            "distance_meters": distance,
        }));
    }

    results.sort_by(|a, b| {
        let da = a.get("distance_meters").and_then(|v| v.as_f64()).unwrap_or(f64::INFINITY);
        let db = b.get("distance_meters").and_then(|v| v.as_f64()).unwrap_or(f64::INFINITY);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(Json(json!(results)))
}

// ---------- Public: list dioceses ----------

async fn list_dioceses(
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let dioceses = sqlx::query_as::<_, Diocese>(
        "SELECT id, name, bishop_name, website, created_at FROM dioceses ORDER BY name ASC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut result = Vec::new();
    for d in dioceses {
        let counts_row = sqlx::query(
            "SELECT category::text as category, COUNT(*) as count FROM institutions WHERE diocese_id = $1 GROUP BY category",
        )
        .bind(d.id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let mut category_counts = Vec::new();
        let mut total = 0i64;
        for cr in &counts_row {
            let cat: String = cr.try_get("category").unwrap_or_default();
            let cnt: i64 = cr.try_get::<i64, _>("count").unwrap_or(0);
            total += cnt;
            category_counts.push(CategoryCount { category: cat, count: cnt });
        }

        result.push(DioceseWithCounts {
            id: d.id,
            name: d.name,
            bishop_name: d.bishop_name,
            website: d.website,
            institution_count: total,
            category_counts,
        });
    }

    Ok(Json(json!(result)))
}

// ---------- Public: diocese institutions grouped by category ----------

async fn diocese_institutions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let rows = sqlx::query(
        "SELECT category, COUNT(*) as count FROM institutions WHERE diocese_id = $1 GROUP BY category ORDER BY category",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut groups: Vec<Value> = Vec::new();
    for r in rows {
        groups.push(json!({
            "category": r.try_get::<String, _>("category").unwrap_or_default(),
            "count": r.try_get::<i64, _>("count").unwrap_or(0),
        }));
    }

    Ok(Json(json!(groups)))
}

// ---------- Public: fuzzy search ----------

async fn search_institutions(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Value>, StatusCode> {
    let rows = sqlx::query(
        "SELECT id, name, category::text as category, ST_AsText(location) as location, cover_image_url, verified,
                 similarity(name, $1) as sim
          FROM institutions
          WHERE name % $1
          ORDER BY sim DESC, name ASC
          LIMIT 50",
    )
    .bind(params.q)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let results: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.try_get::<Uuid, _>("id").unwrap_or(Uuid::nil()),
                "name": r.try_get::<String, _>("name").unwrap_or_default(),
                "category": r.try_get::<String, _>("category").unwrap_or_default(),
                "location": r.try_get::<Option<String>, _>("location").unwrap_or(None),
                "cover_image_url": r.try_get::<Option<String>, _>("cover_image_url").unwrap_or(None),
                "verified": r.try_get::<bool, _>("verified").unwrap_or(false),
            })
        })
        .collect();

    Ok(Json(json!(results)))
}

// ---------- Public: create institution submission ----------

async fn create_institution_submission(
    State(state): State<AppState>,
    Json(payload): Json<NewInstitutionSubmission>,
) -> Result<Json<Value>, StatusCode> {
    if payload.proposed_name.trim().is_empty()
        || !valid_category(&payload.proposed_category)
        || !valid_coordinates(payload.proposed_lat, payload.proposed_lng)
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let loc = match (payload.proposed_lat, payload.proposed_lng) {
        (Some(lat), Some(lng)) => Some(format!(
            "ST_SetSRID(ST_MakePoint({}, {}), 4326)::geography",
            lng, lat
        )),
        _ => None,
    };

    sqlx::query(
        "INSERT INTO community_submissions
            (submission_type, status, submitter_name, submitter_email, submitter_phone,
             proposed_name, proposed_category, proposed_diocese, proposed_deanery,
             proposed_lat, proposed_lng, proposed_address_notes, proposed_phone,
             proposed_priest, additional_notes, submitted_at)
         VALUES ('new_institution', 'pending', $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW())",
    )
    .bind(&payload.submitter_name)
    .bind(&payload.submitter_email)
    .bind(&payload.submitter_phone)
    .bind(&payload.proposed_name)
    .bind(&payload.proposed_category)
    .bind(&payload.proposed_diocese)
    .bind(&payload.proposed_deanery)
    .bind(payload.proposed_lat)
    .bind(payload.proposed_lng)
    .bind(&payload.proposed_address_notes)
    .bind(&payload.proposed_phone)
    .bind(&payload.proposed_priest)
    .bind(&payload.additional_notes)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "ok": true,
        "message": "Thank you — our team will review and add your submission."
    })))
}

// ---------- Public: create image submission ----------

async fn create_image_submission(
    State(_state): State<AppState>,
    Path(institution_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let _ = institution_id;
    Err(StatusCode::NOT_IMPLEMENTED)
}

async fn create_correction_submission(
    State(state): State<AppState>,
    Json(payload): Json<CorrectionSubmission>,
) -> Result<Json<Value>, StatusCode> {
    if payload.correction_field.trim().is_empty()
        || payload.correction_value.trim().is_empty()
        || payload.correction_field.len() > 100
        || payload.correction_value.len() > 5000
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM institutions WHERE id = $1)",
    )
    .bind(payload.institution_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !exists {
        return Err(StatusCode::NOT_FOUND);
    }

    sqlx::query(
        "INSERT INTO community_submissions
         (submission_type, status, submitter_name, submitter_email, submitter_phone,
          institution_id, correction_field, correction_value, additional_notes, submitted_at)
         VALUES ('correction', 'pending', $1, $2, $3, $4, $5, $6, $7, NOW())",
    )
    .bind(payload.submitter_name)
    .bind(payload.submitter_email)
    .bind(payload.submitter_phone)
    .bind(payload.institution_id)
    .bind(payload.correction_field)
    .bind(payload.correction_value)
    .bind(payload.additional_notes)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({"ok": true, "message": "Correction submitted for review."})))
}

fn valid_category(category: &str) -> bool {
    matches!(category, "parish" | "outstation" | "mission" | "school" | "orphanage" | "shrine" | "office")
}

fn valid_coordinates(lat: Option<f64>, lng: Option<f64>) -> bool {
    match (lat, lng) {
        (Some(lat), Some(lng)) => lat.is_finite() && lng.is_finite() && (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lng),
        (None, None) => true,
        _ => false,
    }
}

async fn admin_login(
    State(state): State<AppState>,
    Json(payload): Json<AdminLogin>,
) -> Result<Json<Value>, StatusCode> {
    let username = std::env::var("ADMIN_USERNAME").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let password = std::env::var("ADMIN_PASSWORD").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if payload.username != username || payload.password != password {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let claims = Claims {
        sub: payload.username,
        exp: (Utc::now() + chrono::Duration::hours(8)).timestamp() as usize,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({"token": token})))
}

fn require_admin(headers: &HeaderMap, state: &AppState) -> Result<String, StatusCode> {
    let value = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or_default();
    let token = value.strip_prefix("Bearer ").ok_or(StatusCode::UNAUTHORIZED)?;
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    ).map_err(|_| StatusCode::UNAUTHORIZED)?;
    Ok(token_data.claims.sub)
}

// ---------- Admin: list submissions ----------

async fn list_submissions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&headers, &state)?;
    let rows = sqlx::query(
        "SELECT id, submission_type, status, submitter_name, submitter_email, proposed_name,
                 proposed_category, institution_id, image_s3_key, image_url, image_caption,
                 reviewed_by, reviewed_at, admin_notes, submitted_at
          FROM community_submissions ORDER BY submitted_at DESC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let subs: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.try_get::<Uuid, _>("id").unwrap_or(Uuid::nil()),
                "submission_type": r.try_get::<String, _>("submission_type").unwrap_or_default(),
                "status": r.try_get::<String, _>("status").unwrap_or_default(),
                "submitter_name": r.try_get::<Option<String>, _>("submitter_name").unwrap_or(None),
                "submitter_email": r.try_get::<Option<String>, _>("submitter_email").unwrap_or(None),
                "proposed_name": r.try_get::<Option<String>, _>("proposed_name").unwrap_or(None),
                "proposed_category": r.try_get::<Option<String>, _>("proposed_category").unwrap_or(None),
                "institution_id": r.try_get::<Option<Uuid>, _>("institution_id").unwrap_or(None),
                "image_s3_key": r.try_get::<Option<String>, _>("image_s3_key").unwrap_or(None),
                "image_url": r.try_get::<Option<String>, _>("image_url").unwrap_or(None),
                "image_caption": r.try_get::<Option<String>, _>("image_caption").unwrap_or(None),
                "reviewed_by": r.try_get::<Option<String>, _>("reviewed_by").unwrap_or(None),
                "reviewed_at": r.try_get::<Option<NaiveDateTime>, _>("reviewed_at").unwrap_or(None),
                "admin_notes": r.try_get::<Option<String>, _>("admin_notes").unwrap_or(None),
                "submitted_at": r.try_get::<NaiveDateTime, _>("submitted_at").unwrap_or(Utc::now().naive_utc()),
            })
        })
        .collect();

    Ok(Json(json!(subs)))
}

// ---------- Admin: approve submission ----------

async fn approve_submission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let reviewer = require_admin(&headers, &state)?;
    let mut tx = state.pool.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // Get the submission
    let sub = sqlx::query(
        "SELECT submission_type, status, proposed_name, proposed_category, proposed_diocese,
                 proposed_deanery, proposed_lat, proposed_lng, proposed_address_notes,
                 proposed_phone, proposed_priest, institution_id, image_s3_key, image_url,
                 image_caption, submitter_name
          FROM community_submissions WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let stype: String = sub.try_get("submission_type").unwrap_or_default();
    let status: String = sub.try_get("status").unwrap_or_default();
    if status != "pending" {
        return Err(StatusCode::CONFLICT);
    }

    if stype == "new_institution" {
        let name: String = sub.try_get("proposed_name").unwrap_or_default();
        let category: String = sub.try_get("proposed_category").unwrap_or_else(|_| "parish".to_string());
        let diocese_id: Option<Uuid> = sub.try_get("proposed_diocese").ok().and_then(|s: Option<String>| s.and_then(|v| Uuid::parse_str(&v).ok()));
        let lat: Option<f64> = sub.try_get::<Option<f64>, _>("proposed_lat").ok().flatten();
        let lng: Option<f64> = sub.try_get::<Option<f64>, _>("proposed_lng").ok().flatten();
        let address_notes: Option<String> = sub.try_get::<Option<String>, _>("proposed_address_notes").ok().flatten();
        let phone: Option<String> = sub.try_get::<Option<String>, _>("proposed_phone").ok().flatten();
        let priest: Option<String> = sub.try_get::<Option<String>, _>("proposed_priest").ok().flatten();

        let new_id = Uuid::new_v4();

        let location_column = if lat.is_some() && lng.is_some() {
            ", location"
        } else {
            ""
        };
        let location_value = match (lat, lng) {
            (Some(lat), Some(lng)) => format!(
                ", ST_SetSRID(ST_MakePoint({}, {}), 4326)::geography",
                lng, lat
            ),
            _ => String::new(),
        };

        sqlx::query(&format!(
            "INSERT INTO institutions (id, name, category, diocese_id, address_notes, contact_phone, priest_in_charge{} , verified, created_at, updated_at)
             VALUES ($1, $2, $3::institution_category, $4, $5, $6, $7{} , TRUE, NOW(), NOW())",
            location_column,
            location_value
        ))
        .bind(new_id)
        .bind(&name)
        .bind(&category)
        .bind(diocese_id)
        .bind(&address_notes)
        .bind(&phone)
        .bind(&priest)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else if stype == "correction" {
        let institution_id: Uuid = sub.try_get("institution_id").map_err(|_| StatusCode::BAD_REQUEST)?;
        let field: String = sub.try_get("correction_field").map_err(|_| StatusCode::BAD_REQUEST)?;
        let value: String = sub.try_get("correction_value").map_err(|_| StatusCode::BAD_REQUEST)?;
        let allowed = ["name", "address_notes", "contact_phone", "contact_email", "website", "priest_in_charge"];
        if !allowed.contains(&field.as_str()) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let query = format!("UPDATE institutions SET {} = $1, updated_at = NOW() WHERE id = $2", field);
        sqlx::query(&query)
            .bind(value)
            .bind(institution_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let updated = sqlx::query(
        "UPDATE community_submissions SET status = 'approved', reviewed_by = $1, reviewed_at = NOW() WHERE id = $2 AND status = 'pending'",
    )
    .bind(reviewer)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if updated.rows_affected() != 1 {
        return Err(StatusCode::CONFLICT);
    }
    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "ok": true,
        "message": "Submission approved."
    })))
}

// ---------- Admin: reject submission ----------

async fn reject_submission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    axum::extract::Json(notes): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let reviewer = require_admin(&headers, &state)?;
    let admin_notes = notes.get("admin_notes").and_then(|v| v.as_str()).unwrap_or("");
    if admin_notes.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let updated = sqlx::query(
        "UPDATE community_submissions SET status = 'rejected', admin_notes = $1, reviewed_by = $2, reviewed_at = NOW() WHERE id = $3 AND status = 'pending'",
    )
    .bind(admin_notes)
    .bind(reviewer)
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if updated.rows_affected() != 1 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(json!({
        "ok": true,
        "message": "Submission rejected."
    })))
}

// ---------- Admin: update institution ----------

async fn update_institution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<InstitutionUpdate>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&headers, &state)?;
    let mut sets: Vec<String> = Vec::new();
    let mut param_count: i32 = 0;

    macro_rules! set_field {
        ($field:expr, $val:expr) => {
            if let Some(ref _v) = $val {
                param_count += 1;
                sets.push(format!("{} = ${}", $field, param_count));
            }
        };
    }

    let p = &payload;
    set_field!("name", p.name.as_ref());
    if let Some(category) = &p.category {
        if !valid_category(category) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    if p.category.is_some() {
        param_count += 1;
        sets.push(format!("category = ${}::institution_category", param_count));
    }
    set_field!("school_level", p.school_level.as_ref());
    set_field!("diocese_id", p.diocese_id.as_ref());
    set_field!("deanery_id", p.deanery_id.as_ref());
    set_field!("parent_id", p.parent_id.as_ref());
    set_field!("address_notes", p.address_notes.as_ref());
    set_field!("contact_phone", p.contact_phone.as_ref());
    set_field!("contact_email", p.contact_email.as_ref());
    set_field!("website", p.website.as_ref());
    set_field!("priest_in_charge", p.priest_in_charge.as_ref());
    set_field!("verified", p.verified.as_ref());

    if sets.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    param_count += 1;
    sets.push(format!("updated_at = NOW()"));
    let query = format!(
        "UPDATE institutions SET {} WHERE id = ${}",
        sets.join(", "),
        param_count
    );

    let mut qb = sqlx::query(&query);
    // Re-bind in order
    if let Some(v) = &payload.name { qb = qb.bind(v); }
    if let Some(v) = &payload.category { qb = qb.bind(v); }
    if let Some(v) = &payload.school_level { qb = qb.bind(v); }
    if let Some(v) = payload.diocese_id { qb = qb.bind(v); }
    if let Some(v) = payload.deanery_id { qb = qb.bind(v); }
    if let Some(v) = payload.parent_id { qb = qb.bind(v); }
    if let Some(v) = &payload.address_notes { qb = qb.bind(v); }
    if let Some(v) = &payload.contact_phone { qb = qb.bind(v); }
    if let Some(v) = &payload.contact_email { qb = qb.bind(v); }
    if let Some(v) = &payload.website { qb = qb.bind(v); }
    if let Some(v) = &payload.priest_in_charge { qb = qb.bind(v); }
    if let Some(v) = payload.verified { qb = qb.bind(v); }
    qb = qb.bind(id);

    qb.execute(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "ok": true,
        "message": "Institution updated."
    })))
}

// ---------- Admin: upload image ----------

async fn admin_upload_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&headers, &state)?;
    Err(StatusCode::NOT_IMPLEMENTED)
}

// ---------- Admin: delete image ----------

async fn delete_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((_institution_id, _image_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&headers, &state)?;
    Err(StatusCode::NOT_IMPLEMENTED)
}
