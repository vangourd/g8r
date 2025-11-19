use anyhow::{Context, Result};
use log::info;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::db::StateManager;
use crate::controller::Controller;
use crate::stack::StackManager;
use crate::queue::QueueManager;
use crate::modules::aws::{
    AwsStaticSiteModule, S3BucketModule, ACMCertificateModule,
    CloudFrontDistributionModule, IAMUserModule, Route53RecordModule
};
use super::routes::create_router;
use super::handlers::AppStateInner;

pub struct ApiServer {
    state_manager: StateManager,
    host: String,
    port: u16,
}

impl ApiServer {
    pub fn new(state: StateManager, host: String, port: u16) -> Self {
        Self {
            state_manager: state,
            host,
            port,
        }
    }

    pub async fn run(self) -> Result<()> {
        let mut controller = Controller::new(self.state_manager.clone());
        controller.register_module(Arc::new(AwsStaticSiteModule::new()));
        controller.register_module(Arc::new(S3BucketModule::new(self.state_manager.clone())));
        controller.register_module(Arc::new(ACMCertificateModule::new(self.state_manager.clone())));
        controller.register_module(Arc::new(CloudFrontDistributionModule::new(self.state_manager.clone())));
        controller.register_module(Arc::new(IAMUserModule::new(self.state_manager.clone())));
        controller.register_module(Arc::new(Route53RecordModule::new(self.state_manager.clone())));
        
        let stack_manager = StackManager::new(
            self.state_manager.clone(), 
            Arc::new(controller.clone())
        );
        
        stack_manager.start().await
            .context("Failed to start Stack Manager")?;
        
        let queue_manager = QueueManager::new(
            self.state_manager.clone(),
            Arc::new(controller.clone())
        );
        
        queue_manager.start().await
            .context("Failed to start Queue Manager")?;
        
        let app_state = Arc::new(AppStateInner {
            state_manager: self.state_manager,
            controller,
            stack_manager,
            queue_manager,
        });
        
        let app = create_router(app_state);

        let addr = format!("{}:{}", self.host, self.port);
        info!("Starting API server on {}", addr);

        let listener = TcpListener::bind(&addr)
            .await
            .with_context(|| format!("Failed to bind to {}", addr))?;

        info!("API server listening on http://{}", addr);
        info!("Health check: http://{}/health", addr);
        info!("API endpoints: http://{}/api/v1/rosters, /api/v1/duties, /api/v1/stacks, /api/v1/queues", addr);

        axum::serve(listener, app)
            .await
            .context("Server error")?;

        Ok(())
    }
}
