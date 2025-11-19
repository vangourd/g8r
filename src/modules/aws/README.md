# AWS Module Development Guide

This guide documents common patterns and utilities for developing AWS automation modules in g8r.

## Architecture

AWS modules implement the `AutomationModule` trait defined in `src/modules/mod.rs`. Each module:
- Wraps AWS SDK operations with g8r-specific logic
- Handles eventual consistency with retries
- Validates duty specifications
- Returns standardized JSON outputs
- Implements idempotent operations

## Module Structure

```
src/modules/aws/
├── utils.rs                    # Shared AWS utilities
├── clients/                    # AWS SDK client wrappers
│   ├── traits.rs              # Operation traits
│   ├── s3.rs                  # S3 operations
│   ├── acm.rs                 # ACM operations
│   └── ...
├── s3_bucket.rs               # S3Bucket automation module
├── acm_certificate.rs         # ACMCertificate automation module
└── ...
```

## Common Utilities (`utils.rs`)

### Retry with Exponential Backoff

AWS services exhibit eventual consistency. Use `retry_with_backoff()` when:
- Creating resources and immediately reading them
- Waiting for propagation (IAM, ACM, etc.)
- Handling transient API errors

**Example:**
```rust
use crate::modules::aws::utils::retry_with_backoff;

let bucket_info = retry_with_backoff(
    || s3.get_bucket_info(bucket_name),
    10,
    "fetch S3 bucket info",
).await?;
```

**Behavior:**
- Attempts: 10 (configurable)
- Delays: 2s → 4s → 8s → 16s → 30s (max)
- Logs progress on retries
- Returns error after max attempts

### AWS Client Factories

Use helper functions to initialize AWS SDK clients from roster configuration:

```rust
use crate::modules::aws::utils::{aws_s3_client, aws_acm_client};

// Standard region from roster
let s3 = aws_s3_client(roster).await?;

// Override region (e.g., ACM for CloudFront requires us-east-1)
let acm = aws_acm_client(roster, Some("us-east-1")).await?;
```

**Available clients:**
- `aws_s3_client(roster)` - S3
- `aws_acm_client(roster, region_override)` - ACM
- `aws_route53_client(roster)` - Route53
- `aws_iam_client(roster)` - IAM
- `aws_cloudfront_client(roster)` - CloudFront

**Roster requirements:**
```json
{
  "connection": {
    "region": "us-east-1"
  }
}
```

**Region recommendation:** Use `us-east-1` for all AWS stacks that include CloudFront distributions, as ACM certificates for CloudFront **must** be in `us-east-1`. While other services can use any region, standardizing on `us-east-1` simplifies configuration and avoids cross-region complexity.

## Module Implementation Pattern

### 1. Module Struct

```rust
pub struct MyServiceModule {
    state: StateManager,
}

impl MyServiceModule {
    pub fn new(state: StateManager) -> Self {
        Self { state }
    }
}
```

### 2. Implement AutomationModule Trait

```rust
#[async_trait]
impl AutomationModule for MyServiceModule {
    fn name(&self) -> &str {
        "my-service"
    }

    fn supported_duty_types(&self) -> Vec<&str> {
        vec!["MyService"]
    }

    fn required_roster_traits(&self) -> Vec<&str> {
        vec!["cloud-provider", "aws"]
    }

    async fn validate(&self, _roster: &Roster, duty: &Duty) -> Result<()> {
        // Validate required spec fields
        if duty.spec.get("required_field").is_none() {
            anyhow::bail!("MyService duty requires 'required_field' in spec");
        }
        Ok(())
    }

    async fn apply(&self, roster: &Roster, duty: &Duty) -> Result<JsonValue> {
        // Implementation
    }

    async fn destroy(&self, roster: &Roster, duty: &Duty) -> Result<()> {
        // Cleanup implementation
    }
}
```

### 3. Apply Method Structure

```rust
async fn apply(&self, roster: &Roster, duty: &Duty) -> Result<JsonValue> {
    let spec = &duty.spec;
    
    // Extract spec fields
    let resource_name = spec["name"].as_str()
        .ok_or_else(|| anyhow::anyhow!("name is required"))?;
    
    // Initialize client
    let client = aws_myservice_client(roster).await?;
    
    // Check if resource exists (idempotency)
    let existing_arn = duty.status.as_ref()
        .and_then(|s| s.get("outputs"))
        .and_then(|o| o.get("arn"))
        .and_then(|v| v.as_str());
    
    let resource_arn = if let Some(arn) = existing_arn {
        info!("Resource already exists: {}", arn);
        
        // Check current state with retry
        retry_with_backoff(
            || client.describe_resource(arn),
            5,
            "describe resource",
        ).await?;
        
        arn.to_string()
    } else {
        info!("Creating new resource: {}", resource_name);
        
        // Create resource
        let arn = client.create_resource(resource_name).await?;
        info!("Resource created: {}", arn);
        
        arn
    };
    
    // Return standardized output
    Ok(json!({
        "phase": "deployed",
        "message": format!("Resource '{}' deployed", resource_name),
        "outputs": {
            "arn": resource_arn,
            "name": resource_name,
        }
    }))
}
```

## Output Format Standards

All `apply()` methods return JSON with this structure:

```json
{
  "phase": "deployed|pending_validation|pending",
  "message": "Human-readable status message",
  "outputs": {
    "arn": "resource ARN (if applicable)",
    "id": "resource ID",
    "key": "value",
    ...
  }
}
```

**Phases:**
- `deployed` - Resource fully operational
- `pending_validation` - Waiting for validation (e.g., ACM certificate)
- `pending` - Waiting for dependencies or propagation

## Error Handling

### Use anyhow for Errors

```rust
use anyhow::{Result, Context};

let value = operation()
    .await
    .context("Failed to perform operation")?;
```

### Retry Transient Errors

AWS SDK errors may be transient. Wrap operations in `retry_with_backoff()`:

```rust
retry_with_backoff(
    || client.api_call(),
    5,
    "API operation",
).await?;
```

### Handle Missing Resources Gracefully

```rust
async fn destroy(&self, roster: &Roster, duty: &Duty) -> Result<()> {
    let resource_id = duty.status.as_ref()
        .and_then(|s| s.get("outputs"))
        .and_then(|o| o.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No resource ID in duty status"))?;
    
    let client = aws_myservice_client(roster).await?;
    
    // Check if resource exists
    if client.get_resource(resource_id).await?.is_none() {
        info!("Resource '{}' does not exist, skipping deletion", resource_id);
        return Ok(());
    }
    
    // Delete resource
    info!("Deleting resource '{}'", resource_id);
    client.delete_resource(resource_id).await?;
    
    Ok(())
}
```

## Testing Strategies

### Unit Tests

Test module logic with mocked AWS clients:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_validate_missing_field() {
        let module = MyServiceModule::new(mock_state());
        let duty = Duty {
            spec: json!({}),
            ..Default::default()
        };
        
        let result = module.validate(&mock_roster(), &duty).await;
        assert!(result.is_err());
    }
}
```

### Integration Tests

Test against real AWS resources in a development environment:

```bash
export AWS_PROFILE=dev
export DATABASE_URL=postgresql://...
cargo test --test integration_tests -- --nocapture
```

## Common Patterns

### Region Overrides

Some AWS services require specific regions:

```rust
// ACM certificates for CloudFront must be in us-east-1
let acm = aws_acm_client(roster, Some("us-east-1")).await?;
```

### Dependency Management

Use `depends_on` in duty specs for ordering:

```json
{
  "duties": {
    "cert": {
      "duty_type": "ACMCertificate",
      "spec": {...}
    },
    "cdn": {
      "duty_type": "CloudFrontDistribution",
      "depends_on": ["cert"],
      "spec": {
        "certificate_arn": "${cert.outputs.arn}"
      }
    }
  }
}
```

### Waiting for Validation

Some resources require validation (e.g., ACM certificates):

```rust
if existing_arn.is_some() {
    info!("Checking validation status...");
    match retry_with_backoff(
        || client.check_validated(arn),
        60,  // More attempts for long-running validation
        "certificate validation",
    ).await {
        Ok(_) => {
            return Ok(json!({
                "phase": "deployed",
                "message": "Resource validated",
                "outputs": {...}
            }));
        }
        Err(_) => {
            info!("Validation still pending");
        }
    }
}

Ok(json!({
    "phase": "pending_validation",
    "message": "Waiting for validation",
    "outputs": {...}
}))
```

## Client Trait Pattern

Separate AWS SDK calls into operation traits for testability:

```rust
// src/modules/aws/clients/traits.rs
#[async_trait]
pub trait MyServiceOperations {
    async fn create_resource(&self, name: &str) -> Result<String>;
    async fn get_resource(&self, id: &str) -> Result<Option<JsonValue>>;
    async fn delete_resource(&self, id: &str) -> Result<()>;
}

// src/modules/aws/clients/myservice.rs
pub struct MyServiceModule {
    client: MyServiceClient,
}

#[async_trait]
impl MyServiceOperations for MyServiceModule {
    async fn create_resource(&self, name: &str) -> Result<String> {
        let result = self.client
            .create_resource()
            .name(name)
            .send()
            .await
            .context("Failed to create resource")?;
        
        let id = result.resource_id()
            .context("No resource ID in response")?;
        
        Ok(id.to_string())
    }
    
    // ...
}
```

## Examples

See existing modules for reference:
- **S3Bucket** (`s3_bucket.rs`) - Simple resource with retry
- **ACMCertificate** (`acm_certificate.rs`) - Validation waiting
- **CloudFrontDistribution** (`cloudfront_distribution.rs`) - Dependencies
- **Route53Record** (`route53_record.rs`) - DNS propagation
- **IAMUser** (`iam_user.rs`) - Access keys and policies

## Future: WASM Modules

These patterns are Rust-specific conveniences. Future WASM-based modules will:
- Implement the same `AutomationModule` trait interface (via FFI)
- Use their language's AWS SDK (boto3, AWS SDK for Go, etc.)
- Implement equivalent retry/client helpers in their language
- Communicate via JSON serialization

The `README.md` pattern should be replicated for each language SDK.
