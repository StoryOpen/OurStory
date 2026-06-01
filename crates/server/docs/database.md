# Database

Postgres via `sqlx`. The DB layer is in `server-core/src/db/` so game logic can be tested with a mock implementation.

## Structure

```
server-core/src/db/
├── mod.rs         # Database trait
├── postgres.rs    # PostgresDb impl
└── models.rs      # Row structs (Account, CharacterRow)
```

`server-core/migrations/0001_init.sql` is the initial schema, run by `PostgresDb::run_migrations`.

## The Database Trait

```rust
#[async_trait]
pub trait Database: Send + Sync {
    async fn load_player(&self, id: i32) -> Option<Player>;
    async fn save_player(&self, player: &Player);
}
```

Defined in `server-core`. The `server` binary holds `Arc<dyn Database>` and passes it to the dispatcher, which passes it to handlers. Handlers depend on the trait, not on `PostgresDb` — so unit tests can swap in a mock.

## PostgresDb

Wraps a `sqlx::PgPool`. Two key methods:

- `connect(url)` — creates the pool
- `run_migrations()` — runs `sqlx::migrate!` against `migrations/` directory

Query methods (`load_player`, `save_player`) use `sqlx::query` with manual `Row::get` calls. Not using `sqlx::query!` macros because they require a live DB at compile time.

## Schema

```sql
accounts
├── id            SERIAL PRIMARY KEY
├── username      TEXT UNIQUE NOT NULL
├── password_hash TEXT NOT NULL
├── logged_in     SMALLINT NOT NULL DEFAULT 0
└── banned        BOOLEAN NOT NULL DEFAULT false

characters
├── id          SERIAL PRIMARY KEY
├── account_id  INTEGER REFERENCES accounts(id)
├── world_id    INTEGER NOT NULL
├── name        TEXT UNIQUE NOT NULL
├── level       SMALLINT NOT NULL DEFAULT 1
├── job         INTEGER NOT NULL DEFAULT 0
├── hp, max_hp  INTEGER NOT NULL DEFAULT 50
├── mp, max_mp  INTEGER NOT NULL DEFAULT 50
├── exp         BIGINT NOT NULL DEFAULT 0
├── meso        BIGINT NOT NULL DEFAULT 0
├── map_id      INTEGER NOT NULL DEFAULT 10000
└── position_x, position_y SMALLINT NOT NULL DEFAULT 0
```

This is a minimal subset. Inventory, skills, keybinds, quests, etc. will be added in later migrations. The schema mirrors the columns currently loaded by `load_player` and `save_player`.

## DB Connection Lifecycle

```
main()
  └─ PostgresDb::connect(&config.db_url)        // pool created, .await on connect
  └─ db.run_migrations()                         // idempotent, runs all pending migrations
  └─ db passed as Arc<dyn Database> to dispatcher
```

The pool is created once at startup. `PgPool` handles connection lifecycle internally — it maintains a small pool of connections, acquires/releases per query, and reconnects on failure.

## Query Patterns

**Load player (called on login, channel switch, world switch):**

```rust
let row = sqlx::query("SELECT ... FROM characters WHERE id = $1")
    .bind(id)
    .fetch_optional(&self.pool)
    .await?;
```

**Save player (called on disconnect, periodic autosave, channel switch):**

```rust
sqlx::query("UPDATE characters SET level = $1, ... WHERE id = $12")
    .bind(player.level)
    // ...
    .execute(&self.pool)
    .await?;
```

`save_player` currently ignores the result (`let _ = ...`). In production this should log on failure.

## Why the Trait Lives in `server-core`

Two reasons:

1. **Testability.** Handlers depend on `&Arc<dyn Database>`. Tests in `server-core` can use a mock without pulling in `sqlx`.
2. **Crate boundary.** `server-core` defines the contract (what the game needs from storage). `server` implements the contract against Postgres. If we add a second storage backend (e.g., Redis cache in front of Postgres), it goes in `server` and the trait stays unchanged.

## Future Work

- **Inventory, skills, keymaps** — separate tables, loaded in `load_player` and saved in `save_player`
- **Autosave timer** — a tokio interval task that periodically calls `save_player` for all logged-in players
- **Connection pool tuning** — `PgPool::connect` uses defaults (10 connections). Need to size for the actual load.
- **Transactions** — multi-row updates (e.g., save player + save inventory + save skills) should be in a single `pool.begin().await?` transaction
