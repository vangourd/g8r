use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};

use super::models::*;
use crate::utils::{Roster, Duty, RosterSelector};

#[derive(Clone)]
pub struct StateManager {
    pool: PgPool,
}

impl StateManager {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        
        Ok(Self { pool })
    }
    
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }


    // Roster CRUD operations
    pub async fn create_roster(&self, roster: Roster) -> Result<Roster> {
        let row = sqlx::query_as::<_, Roster>(
            r#"
            INSERT INTO rosters (name, roster_type, traits, connection, auth, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (name) DO UPDATE SET
                roster_type = EXCLUDED.roster_type,
                traits = EXCLUDED.traits,
                connection = EXCLUDED.connection,
                auth = EXCLUDED.auth,
                metadata = EXCLUDED.metadata,
                updated_at = NOW()
            RETURNING *
            "#
        )
        .bind(&roster.name)
        .bind(&roster.roster_type)
        .bind(sqlx::types::Json(&roster.traits))
        .bind(&roster.connection)
        .bind(&roster.auth)
        .bind(&roster.metadata)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row)
    }

    pub async fn get_roster_by_name(&self, name: &str) -> Result<Roster> {
        let row = sqlx::query_as::<_, Roster>(
            "SELECT * FROM rosters WHERE name = $1"
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row)
    }

    pub async fn list_rosters(&self) -> Result<Vec<Roster>> {
        let rows = sqlx::query_as::<_, Roster>(
            "SELECT * FROM rosters ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }

    pub async fn find_rosters_by_traits(&self, traits: &[&str]) -> Result<Vec<Roster>> {
        let traits_json = serde_json::json!(traits);
        let rows = sqlx::query_as::<_, Roster>(
            "SELECT * FROM rosters WHERE traits @> $1::jsonb"
        )
        .bind(&traits_json)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }

    pub async fn update_roster(&self, roster: &Roster) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE rosters
            SET roster_type = $1, traits = $2, connection = $3, auth = $4, metadata = $5
            WHERE name = $6
            "#
        )
        .bind(&roster.roster_type)
        .bind(sqlx::types::Json(&roster.traits))
        .bind(&roster.connection)
        .bind(&roster.auth)
        .bind(&roster.metadata)
        .bind(&roster.name)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    pub async fn store_secret(&self, name: &str, value: &str, description: Option<&str>) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO secrets (name, value, description)
            VALUES ($1, $2, $3)
            ON CONFLICT (name) DO UPDATE SET value = $2, description = $3
            "#
        )
        .bind(name)
        .bind(value)
        .bind(description)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    pub async fn delete_roster(&self, name: &str) -> Result<()> {
        sqlx::query("DELETE FROM rosters WHERE name = $1")
            .bind(name)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }

    // Duty CRUD operations
    pub async fn create_duty(&self, duty: Duty) -> Result<Duty> {
        let row = sqlx::query_as::<_, Duty>(
            r#"
            INSERT INTO duties (name, duty_type, backend, roster_selector, spec, status, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(&duty.name)
        .bind(&duty.duty_type)
        .bind(&duty.backend)
        .bind(&duty.roster_selector)
        .bind(&duty.spec)
        .bind(&duty.status)
        .bind(&duty.metadata)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row)
    }

    pub async fn upsert_duty(&self, duty: Duty) -> Result<Duty> {
        let row = sqlx::query_as::<_, Duty>(
            r#"
            INSERT INTO duties (name, duty_type, backend, roster_selector, spec, status, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (name) 
            DO UPDATE SET
                duty_type = EXCLUDED.duty_type,
                backend = EXCLUDED.backend,
                roster_selector = EXCLUDED.roster_selector,
                spec = EXCLUDED.spec,
                status = EXCLUDED.status,
                metadata = EXCLUDED.metadata,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *
            "#
        )
        .bind(&duty.name)
        .bind(&duty.duty_type)
        .bind(&duty.backend)
        .bind(&duty.roster_selector)
        .bind(&duty.spec)
        .bind(&duty.status)
        .bind(&duty.metadata)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row)
    }

    pub async fn get_duty_by_name(&self, name: &str) -> Result<Duty> {
        let row = sqlx::query_as::<_, Duty>(
            "SELECT * FROM duties WHERE name = $1"
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row)
    }

    pub async fn list_duties(&self) -> Result<Vec<Duty>> {
        let rows = sqlx::query_as::<_, Duty>(
            "SELECT * FROM duties ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }

    pub async fn list_duties_by_type(&self, duty_type: &str) -> Result<Vec<Duty>> {
        let rows = sqlx::query_as::<_, Duty>(
            "SELECT * FROM duties WHERE duty_type = $1 ORDER BY name"
        )
        .bind(duty_type)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }

    pub async fn list_duties_by_backend(&self, backend: &str) -> Result<Vec<Duty>> {
        let rows = sqlx::query_as::<_, Duty>(
            "SELECT * FROM duties WHERE backend = $1 ORDER BY name"
        )
        .bind(backend)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }

    pub async fn update_duty_status(&self, name: &str, status: serde_json::Value) -> Result<()> {
        sqlx::query("UPDATE duties SET status = $1 WHERE name = $2")
            .bind(&status)
            .bind(name)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }

    pub async fn delete_duty(&self, name: &str) -> Result<()> {
        sqlx::query("DELETE FROM duties WHERE name = $1")
            .bind(name)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }

    pub async fn match_roster_for_duty(&self, duty: &Duty) -> Result<Roster> {
        
        let selector: RosterSelector = serde_json::from_value(duty.roster_selector.clone())?;
        
        let mut query_str = String::from("SELECT * FROM rosters WHERE 1=1");
        
        if let Some(ref traits) = selector.traits {
            query_str.push_str(" AND traits @> $1::jsonb");
        }
        
        if let Some(ref roster_type) = selector.roster_type {
            if selector.traits.is_some() {
                query_str.push_str(" AND roster_type = $2");
            } else {
                query_str.push_str(" AND roster_type = $1");
            }
        }
        
        query_str.push_str(" LIMIT 1");
        
        let row = if let Some(ref traits) = selector.traits {
            let traits_json = serde_json::json!(traits);
            if let Some(ref roster_type) = selector.roster_type {
                sqlx::query_as::<_, Roster>(&query_str)
                    .bind(&traits_json)
                    .bind(roster_type)
                    .fetch_one(&self.pool)
                    .await?
            } else {
                sqlx::query_as::<_, Roster>(&query_str)
                    .bind(&traits_json)
                    .fetch_one(&self.pool)
                    .await?
            }
        } else if let Some(ref roster_type) = selector.roster_type {
            sqlx::query_as::<_, Roster>(&query_str)
                .bind(roster_type)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_as::<_, Roster>(&query_str)
                .fetch_one(&self.pool)
                .await?
        };
        
        Ok(row)
    }

    pub async fn record_duty_execution(&self, duty_name: &str, status: &str) -> Result<()> {
        let duty = self.get_duty_by_name(duty_name).await?;
        
        sqlx::query(
            "INSERT INTO duty_executions (duty_id, status, started_at) VALUES ($1, $2, NOW())"
        )
        .bind(duty.id)
        .bind(status)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    pub async fn get_duty_execution_history(&self, duty_name: &str) -> Result<Vec<DutyExecution>> {
        let duty = self.get_duty_by_name(duty_name).await?;
        
        let rows = sqlx::query_as::<_, DutyExecution>(
            "SELECT * FROM duty_executions WHERE duty_id = $1 ORDER BY started_at DESC"
        )
        .bind(duty.id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }


    pub async fn create_stack(&self, stack: Stack) -> Result<Stack> {
        let row = sqlx::query_as::<_, Stack>(
            r#"
            INSERT INTO stacks (name, source_type, source_config, config_path, reconcile_interval, status, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(&stack.name)
        .bind(&stack.source_type)
        .bind(&stack.source_config)
        .bind(&stack.config_path)
        .bind(&stack.reconcile_interval)
        .bind(&stack.status)
        .bind(&stack.metadata)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row)
    }

    pub async fn list_stacks(&self) -> Result<Vec<Stack>> {
        let rows = sqlx::query_as::<_, Stack>(
            "SELECT * FROM stacks ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows)
    }

    pub async fn get_stack_by_name(&self, name: &str) -> Result<Stack> {
        let row = sqlx::query_as::<_, Stack>(
            "SELECT * FROM stacks WHERE name = $1"
        )
        .bind(name)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row)
    }

    pub async fn update_stack_sync(&self, name: &str, version: &str, status: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE stacks 
            SET last_sync_at = NOW(), last_sync_version = $1, status = $2
            WHERE name = $3
            "#
        )
        .bind(version)
        .bind(status)
        .bind(name)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    pub async fn update_stack_status(&self, name: &str, status: &str) -> Result<()> {
        sqlx::query(
            "UPDATE stacks SET status = $1 WHERE name = $2"
        )
        .bind(status)
        .bind(name)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    pub async fn delete_stack(&self, name: &str) -> Result<()> {
        sqlx::query("DELETE FROM stacks WHERE name = $1")
            .bind(name)
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
}
