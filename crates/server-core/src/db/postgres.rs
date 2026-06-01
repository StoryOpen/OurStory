use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::Row;

use protocol::types::{Job, MapId, PlayerId, Position, WorldId};
use std::collections::HashSet;

use crate::player::Player;

pub struct PostgresDb {
    pool: PgPool,
}

impl PostgresDb {
    pub async fn connect(url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(url).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn run_migrations(&self) -> Result<(), sqlx::migrate::MigrateError> {
        let migrator = sqlx::migrate::Migrator::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations"),
        )
        .await?;
        migrator.run(&self.pool).await
    }
}

#[async_trait]
impl super::Database for PostgresDb {
    async fn load_player(&self, id: i32) -> Option<Player> {
        let row = sqlx::query(
            "SELECT id, name, job, level, map_id, hp, max_hp, mp, max_mp, \
             exp, meso, world_id, position_x, position_y \
             FROM characters WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .ok()??;

        Some(Player {
            id: PlayerId(row.get::<i32, _>("id")),
            name: row.get("name"),
            job: match row.get::<i32, _>("job") {
                1 => Job::Warrior,
                2 => Job::Mage,
                3 => Job::Bowman,
                4 => Job::Thief,
                5 => Job::Pirate,
                _ => Job::Beginner,
            },
            level: row.get("level"),
            map_id: MapId(row.get("map_id")),
            position: Position::new(row.get("position_x"), row.get("position_y")),
            hp: row.get("hp"),
            max_hp: row.get("max_hp"),
            mp: row.get("mp"),
            max_mp: row.get("max_mp"),
            exp: row.get("exp"),
            meso: row.get("meso"),
            world_id: WorldId(row.get("world_id")),
            buffs: Vec::new(),
            mount: None,
            active_quests: Vec::new(),
            completed_quests: HashSet::new(),
        })
    }

    async fn save_player(&self, player: &Player) {
        let job = match player.job {
            Job::Beginner => 0,
            Job::Warrior => 1,
            Job::Mage => 2,
            Job::Bowman => 3,
            Job::Thief => 4,
            Job::Pirate => 5,
        };

        let _ = sqlx::query(
            "UPDATE characters SET level = $1, job = $2, hp = $3, max_hp = $4, \
             mp = $5, max_mp = $6, exp = $7, meso = $8, map_id = $9, \
             position_x = $10, position_y = $11 \
             WHERE id = $12",
        )
        .bind(player.level)
        .bind(job)
        .bind(player.hp)
        .bind(player.max_hp)
        .bind(player.mp)
        .bind(player.max_mp)
        .bind(player.exp)
        .bind(player.meso)
        .bind(player.map_id.0)
        .bind(player.position.x)
        .bind(player.position.y)
        .bind(player.id.0)
        .execute(&self.pool)
        .await;
    }
}
