use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(sqlx::FromRow, Serialize)]
pub struct Institution {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    pub school_level: Option<String>,
    pub diocese_id: Option<Uuid>,
    pub deanery_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub location: Option<String>,
    pub address_notes: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub website: Option<String>,
    pub priest_in_charge: Option<String>,
    pub cover_image_url: Option<String>,
    pub verified: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct InstitutionDetail {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    pub school_level: Option<String>,
    pub diocese_id: Option<Uuid>,
    pub deanery_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub location: Option<String>,
    pub address_notes: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub website: Option<String>,
    pub priest_in_charge: Option<String>,
    pub cover_image_url: Option<String>,
    pub verified: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct InstitutionImage {
    pub id: Uuid,
    pub s3_key: String,
    pub url: String,
    pub caption: Option<String>,
    pub submitted_by: Option<String>,
    pub is_cover: bool,
    pub approved: bool,
    pub uploaded_at: NaiveDateTime,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct Diocese {
    pub id: Uuid,
    pub name: String,
    pub bishop_name: Option<String>,
    pub website: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct DioceseWithCounts {
    pub id: Uuid,
    pub name: String,
    pub bishop_name: Option<String>,
    pub website: Option<String>,
    pub institution_count: i64,
    pub category_counts: Vec<CategoryCount>,
}

#[derive(Serialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct Deanery {
    pub id: Uuid,
    pub diocese_id: Uuid,
    pub name: String,
    pub created_at: NaiveDateTime,
}

#[derive(Deserialize)]
pub struct InstitutionListQuery {
    pub category: Option<String>,
    pub diocese_id: Option<Uuid>,
    pub deanery_id: Option<Uuid>,
    pub verified: Option<bool>,
    pub search: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Deserialize)]
pub struct NearbyQuery {
    pub lat: f64,
    pub lng: f64,
    pub radius_km: Option<f64>,
    pub category: Option<String>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Deserialize)]
pub struct NewInstitutionSubmission {
    pub proposed_name: String,
    pub proposed_category: String,
    pub proposed_diocese: Option<String>,
    pub proposed_deanery: Option<String>,
    pub proposed_lat: Option<f64>,
    pub proposed_lng: Option<f64>,
    pub proposed_address_notes: Option<String>,
    pub proposed_phone: Option<String>,
    pub proposed_priest: Option<String>,
    pub submitter_name: Option<String>,
    pub submitter_email: Option<String>,
    pub submitter_phone: Option<String>,
    pub additional_notes: Option<String>,
}

#[derive(Deserialize)]
pub struct CorrectionSubmission {
    pub institution_id: Uuid,
    pub correction_field: String,
    pub correction_value: String,
    pub additional_notes: Option<String>,
    pub submitter_name: Option<String>,
    pub submitter_email: Option<String>,
    pub submitter_phone: Option<String>,
}

#[derive(Deserialize)]
pub struct AdminLogin {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct InstitutionUpdate {
    pub name: Option<String>,
    pub category: Option<String>,
    pub school_level: Option<String>,
    pub diocese_id: Option<Uuid>,
    pub deanery_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub address_notes: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub website: Option<String>,
    pub priest_in_charge: Option<String>,
    pub verified: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

#[derive(Serialize)]
pub struct PaginatedInstitutions {
    pub institutions: Vec<Institution>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(message: String) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(message),
        }
    }
}
