pub use sqlx::types::Uuid;
use sqlx::{FromRow, PgPool, Result};

#[derive(FromRow)]
pub struct Connection {
    pub id: Uuid,
    pub subdomain: String,
    pub proxied_port: String,
    pub proxy_port: Option<String>,
    pub upstream_port: Option<String>,
}

impl Connection {
    pub fn new(subdomain: String, proxied_port: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            subdomain,
            proxied_port,
            proxy_port: None,
            upstream_port: None,
        }
    }

    pub async fn insert(&self, pool: &PgPool) -> Result<()> {
        // language=PostgreSQL
        sqlx::query("INSERT INTO connections (id, subdomain, proxied_port) VALUES ($1, $2, $3)")
            .bind(self.id)
            .bind(&self.subdomain)
            .bind(&self.proxied_port)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_all(pool: &PgPool) -> Result<Vec<Self>> {
        // language=PostgreSQL
        sqlx::query_as("SELECT * FROM connections")
            .fetch_all(pool)
            .await
    }

    pub async fn get(pool: &PgPool, uuid: &Uuid) -> Result<Self> {
        // language=PostgreSQL
        sqlx::query_as("SELECT * FROM connections WHERE id = $1")
            .bind(uuid)
            .fetch_one(pool)
            .await
    }

    pub async fn get_by_subdomain(pool: &PgPool, subdomain: &str) -> Result<Self> {
        // language=PostgreSQL
        sqlx::query_as("SELECT * FROM connections WHERE subdomain = $1")
            .bind(subdomain)
            .fetch_one(pool)
            .await
    }

    pub async fn delete(&self, pool: &PgPool) -> Result<()> {
        // language=PostgreSQL
        sqlx::query("DELETE FROM connections WHERE id = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }
}
