use axum::{
    routing::{delete, get, post},
    Router,
};
use tower_http::trace::TraceLayer;
use tower_http::cors::CorsLayer;

use super::handlers::*;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/rosters", post(create_roster).get(list_rosters))
        .route("/api/v1/duties", post(create_duty))
        .route("/api/v1/duties/:duty_name", get(get_duty))
        .route("/api/v1/duties/:duty_name/reconcile", post(reconcile_duty))
        .route("/api/v1/stacks", post(create_stack).get(list_stacks))
        .route("/api/v1/stacks/:stack_name", get(get_stack).delete(delete_stack))
        .route("/api/v1/stacks/:stack_name/sync", post(sync_stack))
        .route("/api/v1/stacks/:stack_name/destroy", post(destroy_stack))
        .route("/api/v1/queues", post(create_queue).get(list_queues))
        .route("/api/v1/queues/:queue_name", get(get_queue).delete(delete_queue))
        .route("/api/v1/queues/:queue_name/pause", post(pause_queue))
        .route("/api/v1/queues/:queue_name/resume", post(resume_queue))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
