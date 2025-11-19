use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use anyhow::Context;
use chrono::Utc;
use std::sync::Arc;

use crate::db::StateManager;
use crate::controller::Controller;
use crate::stack::StackManager;
use crate::queue::QueueManager;
use super::models::*;

pub struct AppStateInner {
    pub state_manager: StateManager,
    pub controller: Controller,
    pub stack_manager: StackManager,
    pub queue_manager: QueueManager,
}

pub type AppState = Arc<AppStateInner>;

pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_status = match state.state_manager.list_rosters().await {
        Ok(_) => "connected",
        Err(_) => "error",
    };

    let response = HealthResponse {
        status: "ok".to_string(),
        database: db_status.to_string(),
        timestamp: Utc::now(),
    };

    Json(response)
}


fn internal_error(message: String) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "internal_server_error".to_string(),
            message,
        })
    )
}

fn not_found(message: String) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "not_found".to_string(),
            message,
        })
    )
}

pub async fn create_roster(
    State(state): State<AppState>,
    Json(payload): Json<CreateRosterRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let roster = crate::utils::Roster {
        id: None,
        name: payload.name,
        roster_type: payload.roster_type,
        traits: payload.traits,
        connection: payload.connection,
        auth: payload.auth,
        metadata: payload.metadata,
        created_at: None,
        updated_at: None,
    };
    
    let created = state.state_manager.create_roster(roster).await
        .map_err(|e| internal_error(e.to_string()))?;
    
    let response = RosterResponse {
        id: created.id.unwrap(),
        name: created.name,
        roster_type: created.roster_type,
        traits: created.traits,
        connection: created.connection,
        auth: created.auth,
        metadata: created.metadata,
        created_at: created.created_at.unwrap(),
        updated_at: created.updated_at.unwrap(),
    };
    
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_rosters(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let rosters = state.state_manager.list_rosters().await
        .map_err(|e| internal_error(e.to_string()))?;
    
    let responses: Vec<RosterResponse> = rosters.into_iter().map(|r| {
        RosterResponse {
            id: r.id.unwrap(),
            name: r.name,
            roster_type: r.roster_type,
            traits: r.traits,
            connection: r.connection,
            auth: r.auth,
            metadata: r.metadata,
            created_at: r.created_at.unwrap(),
            updated_at: r.updated_at.unwrap(),
        }
    }).collect();
    
    Ok(Json(responses))
}

pub async fn create_duty(
    State(state): State<AppState>,
    Json(payload): Json<CreateDutyRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let duty = crate::utils::Duty {
        id: None,
        name: payload.name,
        duty_type: payload.duty_type,
        backend: payload.backend,
        roster_selector: payload.roster_selector,
        spec: payload.spec,
        status: None,
        metadata: payload.metadata,
        created_at: None,
        updated_at: None,
    };
    
    let created = state.state_manager.create_duty(duty).await
        .map_err(|e| internal_error(e.to_string()))?;
    
    let response = DutyResponse {
        id: created.id.unwrap(),
        name: created.name,
        duty_type: created.duty_type,
        backend: created.backend,
        roster_selector: created.roster_selector,
        spec: created.spec,
        status: created.status,
        metadata: created.metadata,
        created_at: created.created_at.unwrap(),
        updated_at: created.updated_at.unwrap(),
    };
    
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_duty(
    State(state): State<AppState>,
    Path(duty_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let duty = state.state_manager.get_duty_by_name(&duty_name).await
        .map_err(|e| internal_error(e.to_string()))?;
    
    let response = DutyResponse {
        id: duty.id.unwrap(),
        name: duty.name,
        duty_type: duty.duty_type,
        backend: duty.backend,
        roster_selector: duty.roster_selector,
        spec: duty.spec,
        status: duty.status,
        metadata: duty.metadata,
        created_at: duty.created_at.unwrap(),
        updated_at: duty.updated_at.unwrap(),
    };
    
    Ok(Json(response))
}

pub async fn reconcile_duty(
    State(state): State<AppState>,
    Path(duty_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    match state.controller.reconcile_duty(&duty_name).await {
        Ok(_) => {
            let duty = state.state_manager.get_duty_by_name(&duty_name).await
                .map_err(|e| internal_error(e.to_string()))?;
            
            let phase = duty.status
                .and_then(|s| s.get("phase").and_then(|p| p.as_str().map(String::from)))
                .unwrap_or_else(|| "unknown".to_string());
            
            let response = ReconcileResponse {
                duty_name: duty_name.clone(),
                status: phase,
                message: format!("Duty '{}' reconciled successfully", duty_name),
            };
            
            Ok((StatusCode::OK, Json(response)))
        },
        Err(e) => {
            let response = ReconcileResponse {
                duty_name: duty_name.clone(),
                status: "failed".to_string(),
                message: format!("Reconciliation failed: {}", e),
            };
            
            Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(response)))
        }
    }
}

pub async fn create_stack(
    State(state): State<AppState>,
    Json(payload): Json<CreateStackRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let stack = crate::db::models::Stack {
        id: None,
        name: payload.name,
        source_type: payload.source_type,
        source_config: payload.source_config,
        config_path: payload.config_path,
        reconcile_interval: payload.reconcile_interval,
        last_sync_at: None,
        last_sync_version: None,
        status: "pending".to_string(),
        metadata: payload.metadata,
        created_at: None,
        updated_at: None,
    };
    
    let created = state.state_manager.create_stack(stack.clone()).await
        .map_err(|e| internal_error(e.to_string()))?;
    
    state.stack_manager.register_stack(created.clone()).await
        .map_err(|e| internal_error(e.to_string()))?;
    
    let response = StackResponse {
        id: created.id.unwrap(),
        name: created.name,
        source_type: created.source_type,
        source_config: created.source_config,
        config_path: created.config_path,
        reconcile_interval: created.reconcile_interval,
        last_sync_at: created.last_sync_at,
        last_sync_version: created.last_sync_version,
        status: created.status,
        metadata: created.metadata,
        created_at: created.created_at.unwrap(),
        updated_at: created.updated_at.unwrap(),
    };
    
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_stacks(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let stacks = state.state_manager.list_stacks().await
        .map_err(|e| internal_error(e.to_string()))?;
    
    let responses: Vec<StackResponse> = stacks.into_iter().map(|s| {
        StackResponse {
            id: s.id.unwrap(),
            name: s.name,
            source_type: s.source_type,
            source_config: s.source_config,
            config_path: s.config_path,
            reconcile_interval: s.reconcile_interval,
            last_sync_at: s.last_sync_at,
            last_sync_version: s.last_sync_version,
            status: s.status,
            metadata: s.metadata,
            created_at: s.created_at.unwrap(),
            updated_at: s.updated_at.unwrap(),
        }
    }).collect();
    
    Ok(Json(responses))
}

pub async fn get_stack(
    State(state): State<AppState>,
    Path(stack_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let stack = state.state_manager.get_stack_by_name(&stack_name).await
        .map_err(|e| not_found(e.to_string()))?;
    
    let response = StackResponse {
        id: stack.id.unwrap(),
        name: stack.name,
        source_type: stack.source_type,
        source_config: stack.source_config,
        config_path: stack.config_path,
        reconcile_interval: stack.reconcile_interval,
        last_sync_at: stack.last_sync_at,
        last_sync_version: stack.last_sync_version,
        status: stack.status,
        metadata: stack.metadata,
        created_at: stack.created_at.unwrap(),
        updated_at: stack.updated_at.unwrap(),
    };
    
    Ok(Json(response))
}

pub async fn sync_stack(
    State(state): State<AppState>,
    Path(stack_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    match state.stack_manager.sync_stack(&stack_name).await {
        Ok(_) => {
            let response = StackSyncResponse {
                stack_name: stack_name.clone(),
                status: "synced".to_string(),
                message: format!("Stack '{}' synced successfully", stack_name),
            };
            
            Ok((StatusCode::OK, Json(response)))
        },
        Err(e) => {
            let response = StackSyncResponse {
                stack_name: stack_name.clone(),
                status: "failed".to_string(),
                message: format!("Sync failed: {}", e),
            };
            
            Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(response)))
        }
    }
}

pub async fn destroy_stack(
    State(state): State<AppState>,
    Path(stack_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    match state.stack_manager.destroy_stack(&stack_name).await {
        Ok(_) => {
            let response = StackSyncResponse {
                stack_name: stack_name.clone(),
                status: "destroyed".to_string(),
                message: format!("Stack '{}' destroyed successfully", stack_name),
            };
            
            Ok((StatusCode::OK, Json(response)))
        },
        Err(e) => {
            let response = StackSyncResponse {
                stack_name: stack_name.clone(),
                status: "failed".to_string(),
                message: format!("Destroy failed: {}", e),
            };
            
            Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(response)))
        }
    }
}

pub async fn delete_stack(
    State(state): State<AppState>,
    Path(stack_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let stack = state.state_manager.get_stack_by_name(&stack_name).await
        .map_err(|e| not_found(e.to_string()))?;
    
    let stack_id = stack.id.context("Stack missing ID")
        .map_err(|e| internal_error(e.to_string()))?;
    
    state.stack_manager.unregister_stack(stack_id).await
        .map_err(|e| internal_error(e.to_string()))?;
    
    state.state_manager.delete_stack(&stack_name).await
        .map_err(|e| internal_error(e.to_string()))?;
    
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_queue(
    State(_state): State<AppState>,
    Json(_payload): Json<CreateQueueRequest>,
) -> Result<(StatusCode, Json<QueueResponse>), (StatusCode, Json<ErrorResponse>)> {
    Err(internal_error("Queue creation not yet implemented".to_string()))
}

pub async fn list_queues(
    State(_state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let responses: Vec<QueueResponse> = vec![];
    Ok(Json(responses))
}

pub async fn get_queue(
    State(_state): State<AppState>,
    Path(_queue_name): Path<String>,
) -> Result<Json<QueueResponse>, (StatusCode, Json<ErrorResponse>)> {
    Err(not_found("Queue not found".to_string()))
}

pub async fn pause_queue(
    State(_state): State<AppState>,
    Path(queue_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let response = QueueControlResponse {
        queue_name,
        status: "paused".to_string(),
        message: "Queue pause not yet implemented".to_string(),
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn resume_queue(
    State(_state): State<AppState>,
    Path(queue_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let response = QueueControlResponse {
        queue_name,
        status: "active".to_string(),
        message: "Queue resume not yet implemented".to_string(),
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn delete_queue(
    State(_state): State<AppState>,
    Path(_queue_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    Ok(StatusCode::NO_CONTENT)
}
