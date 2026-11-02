use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // 1. Insert dioceses
    let dioceses = [
        "Archdiocese of Harare",
        "Archdiocese of Bulawayo",
        "Diocese of Gweru",
        "Diocese of Masvingo",
        "Diocese of Mutare",
        "Diocese of Hwange",
        "Diocese of Gokwe",
        "Diocese of Chinhoyi",
    ];
    let mut diocese_ids: HashMap<String, Uuid> = HashMap::new();
    for name in &dioceses {
        let existing: Option<Uuid> = sqlx::query("SELECT id FROM dioceses WHERE name = $1")
            .bind(name)
            .fetch_optional(&pool)
            .await?
            .map(|r| r.try_get::<Uuid, _>("id").unwrap_or(Uuid::nil()));

        let id = match existing {
            Some(id) if id != Uuid::nil() => id,
            _ => {
                let new_id = Uuid::new_v4();
                sqlx::query("INSERT INTO dioceses (id, name, created_at) VALUES ($1, $2, NOW())")
                    .bind(new_id)
                    .bind(name)
                    .execute(&pool)
                    .await?;
                new_id
            }
        };
        diocese_ids.insert(name.to_string(), id);
    }
    println!("Inserted {} dioceses", dioceses.len());

    // 2. Process each CSV file
    let csv_files = [
        "seeds/Qwen_csv_20261031_4srfkwd7e.txt",
        "seeds/Qwen_csv_20261031_4tt3rhqt9.txt",
        "seeds/Qwen_csv_20261031_7wdxtcjyh.txt",
        "seeds/Qwen_csv_20261031_c693wh3c2.txt",
        "seeds/Qwen_csv_20261031_kwz01m4f4.txt",
        "seeds/Qwen_csv_20261031_rmea6i6co.txt",
        "seeds/Qwen_csv_20261031_sya7y8obh.txt",
        "seeds/Qwen_csv_20261031_xgjzqoncf.txt",
    ];

    let mut total_inserted = 0i64;
    let mut total_skipped = 0i64;
    let mut deanery_cache: HashMap<(String, String), Uuid> = HashMap::new();

    for csv_file in &csv_files {
        println!("Processing {}", csv_file);
        let content = std::fs::read_to_string(csv_file)?;
        let mut lines = content.lines();
        let _header = lines.next();

        let mut inserted = 0i64;
        let mut skipped = 0i64;

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(8, ',').collect();
            if parts.len() < 8 {
                skipped += 1;
                continue;
            }

            let name = parts[0].trim();
            let category = parts[1].trim().to_lowercase();
            let diocese_name = parts[2].trim();
            let deanery_name = parts[3].trim();
            let _town = parts[4].trim();
            let lat_str = parts[5].trim();
            let lng_str = parts[6].trim();
            let notes = parts[7..].join(",").trim().to_string();

            if name == "Diocesan OVC / Child Protection Programs" {
                skipped += 1;
                continue;
            }

            let diocese_id = match diocese_ids.get(diocese_name) {
                Some(id) => *id,
                None => {
                    println!("  Warning: unknown diocese '{}'", diocese_name);
                    skipped += 1;
                    continue;
                }
            };

            // Look up or create deanery
            let deanery_id_opt = if !deanery_name.is_empty() {
                let cache_key = (diocese_name.to_string(), deanery_name.to_string());
                if let Some(id) = deanery_cache.get(&cache_key) {
                    Some(*id)
                } else {
                    let existing: Option<Uuid> =
                        sqlx::query("SELECT id FROM deaneries WHERE diocese_id = $1 AND name = $2")
                            .bind(diocese_id)
                            .bind(deanery_name)
                            .fetch_optional(&pool)
                            .await?
                            .map(|r| r.try_get::<Uuid, _>("id").unwrap_or(Uuid::nil()));

                    match existing {
                        Some(id) => {
                            deanery_cache.insert(cache_key, id);
                            Some(id)
                        }
                        None => {
                            let id = Uuid::new_v4();
                            sqlx::query(
                                "INSERT INTO deaneries (id, diocese_id, name, created_at) VALUES ($1, $2, $3, NOW())",
                            )
                            .bind(id)
                            .bind(diocese_id)
                            .bind(deanery_name)
                            .execute(&pool)
                            .await?;
                            deanery_cache.insert(cache_key, id);
                            Some(id)
                        }
                    }
                }
            } else {
                None
            };

            let lat: Option<f64> = lat_str.parse().ok();
            let lng: Option<f64> = lng_str.parse().ok();

            let id = Uuid::new_v4();

            let result = sqlx::query(
                "INSERT INTO institutions (id, name, category, diocese_id, deanery_id, location, address_notes, verified, created_at, updated_at)
                 VALUES ($1, $2, $3::institution_category, $4, $5,
                         CASE WHEN $6 IS NOT NULL AND $7 IS NOT NULL
                              THEN ST_SetSRID(ST_MakePoint($6, $7), 4326)::geography
                              ELSE NULL END,
                         $8, FALSE, NOW(), NOW())",
            )
            .bind(id)
            .bind(name)
            .bind(category)
            .bind(diocese_id)
            .bind(deanery_id_opt)
            .bind(lng)
            .bind(lat)
            .bind(if notes.is_empty() { None } else { Some(notes) })
            .execute(&pool)
            .await;

            match result {
                Ok(_) => inserted += 1,
                Err(e) => {
                    println!("  Error inserting '{}': {}", name, e);
                    skipped += 1;
                }
            }
        }

        println!("  Inserted: {}, Skipped: {}", inserted, skipped);
        total_inserted += inserted;
        total_skipped += skipped;
    }

    println!(
        "\nTotal inserted: {}, Total skipped: {}",
        total_inserted, total_skipped
    );
    Ok(())
}
