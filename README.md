# National Index Tracker

National Index Tracker provides a plain HTML, CSS, and JavaScript frontend backed by a Rust API for user accounts, sectors, indices, and public data from data.gov.in.

## Backend stack

- Rust 2024 with Axum
- PostgreSQL 17 with SQLx migrations
- Docker Compose for the frontend, API, migration runner, and database
- Argon2 password hashing and database-backed session tokens
- JSONB payloads for source records
- `tracing` JSON logs written to `NIT_v1/logs/nit.log`

The frontend is available at `http://localhost:8080`. The API is available directly at `http://localhost:3000/api/v1` and through the frontend's same-origin proxy at `http://localhost:8080/api/v1`.

## Quick start

Run commands from the backend directory:

```bash
cd NIT_v1
```

Create a local `.env` file. It is ignored by Git and must not contain values intended for commits:

```dotenv
POSTGRES_USER=app_user
POSTGRES_PASSWORD=change-me
POSTGRES_DB=app_db
DATABASE_URL=postgres://app_user:change-me@127.0.0.1:5433/app_db
DATA_GOV_API_KEY=your-data-gov-in-api-key
```

Build and start the stack:

```bash
docker compose up --build
```

Docker starts PostgreSQL, waits for its health check, runs the SQLx migration container, then starts the backend and Nginx frontend. The database is published on `127.0.0.1:5433`, the API on `127.0.0.1:3000`, and the frontend on `127.0.0.1:8080`. Open `http://localhost:8080` in a browser to use the application. Nginx serves the static files and proxies `/api/v1` requests to the backend so browser requests use the frontend's origin.

Check the service:

```bash
curl http://localhost:3000/api/v1/health
```

Expected response:

```json
{
  "status": "ok",
  "database": "connected"
}
```

Use `docker compose logs -f frontend`, `docker compose logs -f backend`, `docker compose logs -f migrate`, or `docker compose logs -f db` for container output. Backend application logs are also written to `NIT_v1/logs/nit.log`.

## Frontend

The frontend uses static files in `NIT_v1/frontend` and has no Node.js or frontend build step. Docker Compose runs the official Nginx image to serve the files. The frontend container mounts the directory read-only, so edits to HTML, CSS, or JavaScript are visible after refreshing the page.

The UI has registration and login pages, a regular user dashboard, an admin dashboard, sector pages, index data pages, and a signed-out page. It uses the authenticated API for sector and index browsing. The index data page renders the latest payload's field metadata and records as a scrollable table. Admin users also see controls for creating and deleting sectors and indices.

Login redirects according to the API response role. The frontend keeps the access token in `sessionStorage` for the current tab and sends it as a bearer token. The refresh token stays in the API's HTTP-only cookie. Expired access tokens are refreshed and the failed request is retried. Logging out revokes the session and opens the signed-out page.

Start the whole project with `docker compose up --build` from `NIT_v1`, then visit `http://localhost:8080`. To rebuild or restart only the frontend service, use:

```bash
docker compose up -d --build frontend
```

Frontend files are plain static assets. The JavaScript can be syntax-checked with `node --check frontend/app.js`, but Node is not needed to run the frontend.

### Basic user flow

1. Create an account and sign in.
2. Browse the available sectors and open one to see its indices.
3. Open an index to view the description, field names, and latest saved records.
4. Continue using the app while the access token is valid. The frontend refreshes it through the cookie when needed.
5. Sign out to revoke the session.

### Administrator flow

1. Register an account and promote it with the SQL command in Authentication and authorization.
2. Sign in again to receive an administrator access token.
3. Browse the same sectors and index data as regular users.
4. Use the catalog forms to create sectors and indices. Delete controls appear on sector and index pages.

The backend permits one active session per user. If a login attempt reports `Already logged in`, the existing session must be revoked before this browser can sign in. If the original browser is unavailable, an administrator can clear the active session in PostgreSQL:

```sql
BEGIN;

UPDATE user_sessions
SET revoked_at = CURRENT_TIMESTAMP
WHERE user_id = (SELECT id FROM users WHERE email = 'user@example.com')
  AND revoked_at IS NULL;

UPDATE users
SET is_logged_in = FALSE
WHERE email = 'user@example.com';

COMMIT;
```

## Database design

```mermaid
erDiagram
    users ||--o{ user_interests : selects
    interests ||--o{ user_interests : contains
    interests ||--o{ indices : groups
    users ||--o{ user_sessions : owns
    indices ||--o{ index_values : has
    data_sources ||--o{ index_values : supplies

    users {
        bigint id PK
        varchar email UK
        boolean is_admin
        boolean is_logged_in
    }
    user_sessions {
        bigint id PK
        bigint user_id FK
        bytea access_token_hash UK
        bytea refresh_token_hash UK
        timestamptz access_expires_at
        timestamptz refresh_expires_at
        timestamptz revoked_at
    }
    interests {
        integer id PK
        varchar name UK
        text description
    }
    user_interests {
        integer id PK
        bigint user_id FK
        integer interest_id FK
    }
    indices {
        integer id PK
        integer interest_id FK
        varchar name
        text description
    }
    data_sources {
        integer id PK
        varchar name
        text url
        text api_endpoint
    }
    index_values {
        integer id PK
        integer index_id FK
        integer source_id FK
        jsonb payload
        date observed_on
        timestamp fetched_at
    }
```

`interests` represent sectors. `indices` belong to one sector and are unique within that sector. `index_values.payload` holds an entire source response's `records` array as JSONB. Each import creates a new snapshot, and the payload API returns the latest one by observed date, fetch time, and ID.

The current seed migration creates five sectors and five matching indices:

| Sector | Index |
| --- | --- |
| Public Finance | Central GST Collections vs. Estimates |
| Energy | Monthly Crude Oil Processed by Refineries |
| Skill Development | PMKVY Training by State and UT |
| Agriculture | Sugarcane Fair and Remunerative Price |
| Water Resources | PMKSY Watershed Projects and Funding |

The seed migration clears and rebuilds the catalog tables. It is intended for the initial catalog setup. Treat applied migrations as immutable. Add a new migration for every later schema or seed change because SQLx rejects a modified migration checksum.

## Authentication and authorization

Registration is public. Every sector and index route requires an access token:

```http
Authorization: Bearer <access-token>
```

An access token expires after 15 minutes. Login and refresh also issue an HTTP-only, secure refresh-token cookie scoped to `/api/v1/auth`; the refresh token expires after seven days. Refresh rotates both tokens. A user can have one unrevoked session at a time.

Creating or deleting sectors and indices requires `is_admin = TRUE`. There is no API to promote an account, so bootstrap an administrator with PostgreSQL and then log in again:

```sql
UPDATE users
SET is_admin = TRUE
WHERE email = 'admin@example.com';
```

API errors use this shape:

```json
{
  "error": {
    "code": "INDEX_NOT_FOUND",
    "message": "Index not found"
  }
}
```

## API reference

All table paths are relative to `http://localhost:3000/api/v1`. All JSON request bodies use `Content-Type: application/json`.

### Health

| Method | Path | Authentication | Success |
| --- | --- | --- | --- |
| `GET` | `/health` | None | `200` |

### Authentication

| Method | Path | Authentication | Success |
| --- | --- | --- | --- |
| `POST` | `/auth/register` | None | `201` |
| `POST` | `/auth/login` | None | `200` |
| `POST` | `/auth/refresh` | Refresh cookie | `200` |
| `POST` | `/auth/logout` | Access token | `200` |

Register an account. Names must contain 1 to 100 characters, email addresses are normalized to lowercase and validated, and passwords must contain 12 to 128 bytes.

```bash
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"name":"Amrit Shah","email":"amrit@example.com","password":"a-secure-password"}'
```

```json
{
  "data": {
    "id": 1,
    "name": "Amrit Shah",
    "email": "amrit@example.com"
  }
}
```

Log in and save the access token from `data.accessToken`. The response also sets the refresh-token cookie and sends `Cache-Control: no-store`.

```bash
curl -i -c cookies.txt -X POST http://localhost:3000/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"asha@example.com","password":"a-secure-password"}'
```

```json
{
  "data": {
    "message": "Logged in",
    "role": "user",
    "redirectTo": "/dashboard",
    "accessToken": "<64-character-hex-token>"
  }
}
```

Refresh an expired access token with the stored cookie:

```bash
curl -i -b cookies.txt -c cookies.txt -X POST \
  http://localhost:3000/api/v1/auth/refresh
```

The refresh response is `{"data":{"accessToken":"<64-character-hex-token>"}}` and rotates the refresh cookie.

Log out with the access token. The active session is revoked and the refresh cookie is cleared.

```bash
curl -i -X POST http://localhost:3000/api/v1/auth/logout \
  -H 'Authorization: Bearer <access-token>'
```

### Sectors

| Method | Path | Authentication | Role | Success |
| --- | --- | --- | --- | --- |
| `GET` | `/interests` | Access token | Any signed-in user | `200` |
| `POST` | `/interests` | Access token | Admin | `201` |
| `GET` | `/interests/{interest_id}` | Access token | Any signed-in user | `200` |
| `DELETE` | `/interests/{interest_id}` | Access token | Admin | `204` |

Create a sector:

```bash
curl -X POST http://localhost:3000/api/v1/interests \
  -H 'Authorization: Bearer <admin-access-token>' \
  -H 'Content-Type: application/json' \
  -d '{"name":"Health","description":"Public health indicators"}'
```

```json
{
  "id": 6,
  "name": "Health",
  "description": "Public health indicators"
}
```

`GET /interests` returns an array of these objects. `GET /interests/{interest_id}` returns one object. Duplicate names return `409 INTEREST_ALREADY_EXISTS`; missing IDs return `404 INTEREST_NOT_FOUND`.

Deleting a sector deletes its indices through the database foreign key. It returns `409 INTEREST_IN_USE` when stored index values still reference those indices.

### Indices and payloads

| Method | Path | Authentication | Role | Success |
| --- | --- | --- | --- | --- |
| `GET` | `/interests/{interest_id}/indices` | Access token | Any signed-in user | `200` |
| `POST` | `/indices` | Access token | Admin | `201` |
| `GET` | `/indices/{index_id}` | Access token | Any signed-in user | `200` |
| `DELETE` | `/indices/{index_id}` | Access token | Admin | `204` |
| `GET` | `/indices/{index_id}/payload` | Access token | Any signed-in user | `200` |

Create an index:

```bash
curl -X POST http://localhost:3000/api/v1/indices \
  -H 'Authorization: Bearer <admin-access-token>' \
  -H 'Content-Type: application/json' \
  -d '{"interest_id":6,"name":"Hospital Beds","description":"Available beds by state"}'
```

An index response has this shape:

```json
{
  "id": 12,
  "interest_id": 6,
  "name": "Hospital Beds",
  "description": "Available beds by state"
}
```

`GET /interests/{interest_id}/indices` returns an array of index objects and `GET /indices/{index_id}` returns one. Duplicate names within a sector return `409 INDEX_ALREADY_EXISTS`; an unknown index returns `404 INDEX_NOT_FOUND`.

The latest payload endpoint returns the human-readable index description, source field metadata, and the full records array saved in JSONB:

```bash
curl http://localhost:3000/api/v1/indices/1/payload \
  -H 'Authorization: Bearer <access-token>'
```

```json
{
  "description": "State and UT-wise projects sanctioned and funds released",
  "fields": [
    {"id":"state_ut","name":"State/UT","type":"keyword"}
  ],
  "records": [
    {"state_ut":"Andhra Pradesh","central_funds_released___total":45.74}
  ]
}
```

`404 INDEX_PAYLOAD_NOT_FOUND` means the index exists but no data snapshot has been imported yet. `404 INDEX_NOT_FOUND` means the index ID does not exist.

## Importing data.gov.in records

The `collect-data` binary reads `src/features/data/mappings.json`, which maps each seeded sector and index to a data.gov.in resource UUID. It uses the catalog's `api_endpoint` values and the `DATA_GOV_API_KEY` in `.env`.

Run it after Docker has applied migrations and the database is available:

```bash
cd NIT_v1
cargo run --bin collect-data
```

The collector fetches pages of 500 records, validates API status, offsets, counts, totals, and stable field metadata, then opens one database transaction. A successful run:

1. preserves the index's human-written description and appends source field metadata once
2. inserts one JSONB `records` array for every mapped index-source pair
3. uses the current India Standard Time date as `observed_on`

All network fetches finish before the database transaction opens. A failed fetch or database write leaves the import batch unsaved. Running the collector again creates a later snapshot for each mapping.

Set `DATA_MAPPINGS_PATH` to use a different mapping file:

```bash
DATA_MAPPINGS_PATH=path/to/mappings.json cargo run --bin collect-data
```

## User flows

### Basic user

1. Register through `POST /auth/register`.
2. Log in through `POST /auth/login` and save `data.accessToken`. Retain the refresh cookie in the browser or cookie jar.
3. Send the access token to `GET /interests`.
4. Select a sector and request `GET /interests/{interest_id}/indices`.
5. Select an index and request `GET /indices/{index_id}/payload` to render its fields and records.
6. Call `POST /auth/refresh` with the refresh cookie when the access token expires.
7. Call `POST /auth/logout` with the access token when ending the session.

### Administrator

1. Register normally, then set `users.is_admin` with the bootstrap SQL command above.
2. Log in again to obtain an access token reflecting the administrator role. Login responses use `role: "admin"` and `redirectTo: "/admin/dashboard"`.
3. Use `POST /interests` to add a sector and `POST /indices` to add an index under it.
4. Use the data collector to add snapshots for catalog entries mapped to data.gov.in resources.
5. Use `DELETE /indices/{index_id}` or `DELETE /interests/{interest_id}` when safe. Stored payload references may prevent deletion and return a conflict.

## Testing

Run the full suite from `NIT_v1` with a PostgreSQL `DATABASE_URL` available:

```bash
cargo test
```

The suite contains service tests and authentication lifecycle tests. SQLx creates temporary PostgreSQL databases and applies the migration chain to each one. The test-only catalog cleanup migration clears the production seed rows in those temporary databases so fixtures start from an empty catalog.

To rebuild the complete Docker environment after source, dependency, or migration changes:

```bash
docker compose up --build
```

If `migrate` reports that an applied migration was modified, restore the original migration file and create a new migration for the intended change. Rebuild the image afterward so the migration container receives the current migration files.
