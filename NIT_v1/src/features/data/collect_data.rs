use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use sqlx::{PgPool, types::Json};
use std::{env, path::Path};

const MAPPINGS_PATH: &str = "src/features/data/mappings.json";
const PAGE_SIZE: usize = 500;
const MAX_PAGES: usize = 10_000;

#[derive(Debug, Deserialize)]
struct Mapping {
    sector: String,
    index: String,
    resource: String,
}

struct Snapshot {
    index_id: i32,
    source_id: i32,
    fields: Value,
    records: Value,
}

fn integer(value: Option<&Value>, name: &str) -> Result<usize> {
    let value = value.with_context(|| format!("response is missing {name}"))?;
    if let Some(number) = value.as_u64() {
        return usize::try_from(number).context("pagination value is too large");
    }
    if let Some(text) = value.as_str() {
        return text
            .parse()
            .with_context(|| format!("response contains an invalid {name}"));
    }
    bail!("response contains an invalid {name}");
}

fn parse_page(body: Value, expected_offset: usize) -> Result<(usize, Vec<Value>, Value)> {
    if body.get("status").and_then(Value::as_str) != Some("ok") {
        bail!("API response status is not ok");
    }

    let total = integer(body.get("total"), "total")?;
    let offset = integer(body.get("offset"), "offset")?;
    let count = integer(body.get("count"), "count")?;
    let records = body
        .get("records")
        .and_then(Value::as_array)
        .context("response records is missing or not an array")?;
    let fields = body
        .get("field")
        .and_then(Value::as_array)
        .context("response field is missing or not an array")?;

    if offset != expected_offset
        || count != records.len()
        || expected_offset > total
        || records.len() > total.saturating_sub(expected_offset)
    {
        bail!("inconsistent pagination metadata");
    }

    Ok((total, records.clone(), Value::Array(fields.clone())))
}

async fn fetch_snapshot(
    client: &Client,
    api_key: &str,
    endpoint: &str,
    index_id: i32,
    source_id: i32,
) -> Result<Snapshot> {
    let mut records = Vec::new();
    let mut expected_total = None;
    let mut expected_fields = None;

    for _ in 0..MAX_PAGES {
        let offset = records.len();
        let limit = PAGE_SIZE.to_string();
        let offset_text = offset.to_string();
        let response = client
            .get(endpoint)
            .query(&[
                ("api-key", api_key),
                ("format", "json"),
                ("limit", limit.as_str()),
                ("offset", offset_text.as_str()),
            ])
            .send()
            .await
            .context("data.gov.in request failed")?;
        let status = response.status();
        if !status.is_success() {
            bail!("data.gov.in returned HTTP {status}");
        }
        let body = response
            .json::<Value>()
            .await
            .context("data.gov.in returned invalid JSON")?;
        let (total, page_records, fields) = parse_page(body, offset)?;

        if let Some(previous) = expected_total {
            if previous != total {
                bail!("API total changed between pages");
            }
        } else {
            expected_total = Some(total);
        }
        if let Some(previous) = &expected_fields {
            if previous != &fields {
                bail!("API fields changed between pages");
            }
        } else {
            expected_fields = Some(fields);
        }
        if page_records.is_empty() && offset < total {
            bail!("API returned an empty page before all records were fetched");
        }

        records.extend(page_records);
        if records.len() == total {
            return Ok(Snapshot {
                index_id,
                source_id,
                fields: expected_fields.context("API did not return fields")?,
                records: Value::Array(records),
            });
        }
    }

    bail!("page limit reached before all records were fetched");
}

async fn load_mappings(path: &Path) -> Result<Vec<Mapping>> {
    let mappings = serde_json::from_slice::<Vec<Mapping>>(
        &tokio::fs::read(path)
            .await
            .with_context(|| format!("could not read mappings file {}", path.display()))?,
    )
    .context("mappings file contains invalid JSON")?;
    if mappings.is_empty() {
        bail!("mappings file is empty");
    }
    Ok(mappings)
}

async fn resolve_mapping(pool: &PgPool, mapping: &Mapping) -> Result<(i32, i32, String)> {
    let endpoint = format!("https://api.data.gov.in/resource/{}", mapping.resource);
    sqlx::query_as::<_, (i32, i32, String)>(
        "SELECT i.id, ds.id, ds.api_endpoint
         FROM indices i
         JOIN interests s ON s.id = i.interest_id
         JOIN data_sources ds ON ds.api_endpoint = $3
         WHERE s.name = $1 AND i.name = $2",
    )
    .bind(&mapping.sector)
    .bind(&mapping.index)
    .bind(&endpoint)
    .fetch_optional(pool)
    .await?
    .with_context(|| {
        format!(
            "could not resolve sector '{}', index '{}', resource '{}'",
            mapping.sector, mapping.index, mapping.resource
        )
    })
}

async fn run() -> Result<()> {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let api_key = env::var("DATA_GOV_API_KEY").context("DATA_GOV_API_KEY is required")?;
    if api_key.trim().is_empty() {
        bail!("DATA_GOV_API_KEY is empty");
    }

    let mappings_path = env::var("DATA_MAPPINGS_PATH").unwrap_or_else(|_| MAPPINGS_PATH.into());
    let mappings = load_mappings(Path::new(&mappings_path)).await?;
    let pool = PgPool::connect(&database_url)
        .await
        .context("database connection failed")?;
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .context("could not create HTTP client")?;

    let mut snapshots = Vec::with_capacity(mappings.len());
    for mapping in &mappings {
        let (index_id, source_id, endpoint) = resolve_mapping(&pool, mapping).await?;
        eprintln!(
            "Fetching resource {} for index {}",
            mapping.resource, index_id
        );
        snapshots.push(
            fetch_snapshot(&client, &api_key, &endpoint, index_id, source_id)
                .await
                .with_context(|| format!("fetch failed for resource {}", mapping.resource))?,
        );
    }

    let mut transaction = pool.begin().await.context("could not begin transaction")?;
    for snapshot in &snapshots {
        let marker = format!("[data.gov.in fields source_id={}]", snapshot.source_id);
        let fields = serde_json::to_string_pretty(&snapshot.fields)?;
        let description = format!("{marker}\n{fields}");
        sqlx::query(
            "UPDATE indices
             SET description = CASE
                 WHEN position($2 in COALESCE(description, '')) > 0 THEN description
                 WHEN COALESCE(description, '') = '' THEN $3
                 ELSE description || E'\\n\\n' || $3
             END
             WHERE id = $1",
        )
        .bind(snapshot.index_id)
        .bind(&marker)
        .bind(&description)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO index_values (index_id, source_id, payload, observed_on)
             VALUES ($1, $2, $3, (CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Kolkata')::date)",
        )
        .bind(snapshot.index_id)
        .bind(snapshot.source_id)
        .bind(Json(snapshot.records.clone()))
        .execute(&mut *transaction)
        .await?;
    }
    transaction
        .commit()
        .await
        .context("database transaction failed")?;

    for snapshot in snapshots {
        let count = snapshot.records.as_array().map_or(0, Vec::len);
        println!(
            "Saved index_id={} source_id={} records={count}",
            snapshot.index_id, snapshot.source_id
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("collect-data failed: {error:#}");
        std::process::exit(1);
    }
}
