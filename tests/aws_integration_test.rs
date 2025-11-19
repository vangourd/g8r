use anyhow::Result;
use serde_json::json;
use g8r::modules::AutomationModule;
use g8r::modules::aws::s3_bucket::S3BucketModule;
use g8r::modules::aws::route53_record::Route53RecordModule;
use g8r::modules::aws::iam_user::IAMUserModule;
use g8r::utils::{Roster, Duty};
use g8r::db::StateManager;
use std::env;

fn get_test_prefix() -> String {
    format!("g8r-test-{}", chrono::Utc::now().timestamp())
}

async fn init_state_manager() -> Result<StateManager> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://g8r:g8r_dev_password@localhost:5432/g8r_state".to_string());
    StateManager::new(&database_url).await
}

fn create_aws_roster() -> Roster {
    let region = env::var("AWS_REGION").unwrap_or_else(|_| "us-east-2".to_string());
    
    Roster {
        id: Some(1),
        name: "test-aws-roster".to_string(),
        roster_type: "aws-account".to_string(),
        traits: vec!["cloud-provider".to_string(), "aws".to_string()],
        connection: json!({
            "region": region
        }),
        auth: json!({}),
        metadata: Some(json!({})),
        created_at: None,
        updated_at: None,
    }
}

#[tokio::test]
#[ignore = "Integration test - requires AWS credentials"]
async fn test_s3_bucket_create_and_destroy() -> Result<()> {
    let state = init_state_manager().await?;
    let module = S3BucketModule::new(state);
    let roster = create_aws_roster();
    let test_prefix = get_test_prefix();
    let bucket_name = format!("{}-basic", test_prefix);
    
    let duty = Duty {
        id: Some(1),
        name: "test-s3-bucket".to_string(),
        duty_type: "S3Bucket".to_string(),
        backend: "aws".to_string(),
        roster_selector: json!({"traits": ["cloud-provider", "aws"]}),
        spec: json!({
            "bucket_name": bucket_name
        }),
        status: Some(json!({})),
        metadata: Some(json!({})),
        created_at: None,
        updated_at: None,
    };
    
    module.validate(&roster, &duty).await?;
    
    let result = module.apply(&roster, &duty).await?;
    assert_eq!(result["phase"], "deployed");
    assert_eq!(result["outputs"]["bucket_name"], bucket_name);
    
    module.destroy(&roster, &duty).await?;
    
    Ok(())
}

#[tokio::test]
#[ignore = "Integration test - requires AWS credentials"]
async fn test_s3_bucket_with_website_config() -> Result<()> {
    let state = init_state_manager().await?;
    let module = S3BucketModule::new(state);
    let roster = create_aws_roster();
    let test_prefix = get_test_prefix();
    let bucket_name = format!("{}-website", test_prefix);
    
    let duty = Duty {
        id: Some(2),
        name: "test-s3-website".to_string(),
        duty_type: "S3Bucket".to_string(),
        backend: "aws".to_string(),
        roster_selector: json!({"traits": ["cloud-provider", "aws"]}),
        spec: json!({
            "bucket_name": bucket_name,
            "website_config": {
                "index_document": "index.html",
                "error_document": "404.html"
            },
            "public_access": true
        }),
        status: Some(json!({})),
        metadata: Some(json!({})),
        created_at: None,
        updated_at: None,
    };
    
    module.validate(&roster, &duty).await?;
    
    let result = module.apply(&roster, &duty).await?;
    assert_eq!(result["phase"], "deployed");
    assert!(result["outputs"]["website_endpoint"].is_string());
    
    module.destroy(&roster, &duty).await?;
    
    Ok(())
}

#[tokio::test]
#[ignore = "Integration test - requires AWS credentials"]
async fn test_s3_bucket_idempotency() -> Result<()> {
    let state = init_state_manager().await?;
    let module = S3BucketModule::new(state);
    let roster = create_aws_roster();
    let test_prefix = get_test_prefix();
    let bucket_name = format!("{}-idempotent", test_prefix);
    
    let duty = Duty {
        id: Some(3),
        name: "test-s3-idempotent".to_string(),
        duty_type: "S3Bucket".to_string(),
        backend: "aws".to_string(),
        roster_selector: json!({"traits": ["cloud-provider", "aws"]}),
        spec: json!({
            "bucket_name": bucket_name
        }),
        status: Some(json!({})),
        metadata: Some(json!({})),
        created_at: None,
        updated_at: None,
    };
    
    let result1 = module.apply(&roster, &duty).await?;
    let result2 = module.apply(&roster, &duty).await?;
    
    assert_eq!(result1["outputs"]["bucket_name"], result2["outputs"]["bucket_name"]);
    assert_eq!(result1["outputs"]["arn"], result2["outputs"]["arn"]);
    
    module.destroy(&roster, &duty).await?;
    
    Ok(())
}

#[tokio::test]
#[ignore = "Integration test - requires AWS credentials and hosted zone"]
async fn test_route53_cname_record() -> Result<()> {
    let state = init_state_manager().await?;
    let module = Route53RecordModule::new(state);
    let roster = create_aws_roster();
    let test_prefix = get_test_prefix();
    
    let hosted_zone_id = env::var("TEST_HOSTED_ZONE_ID")
        .expect("TEST_HOSTED_ZONE_ID must be set for Route53 tests");
    let test_domain = env::var("TEST_DOMAIN")
        .expect("TEST_DOMAIN must be set for Route53 tests");
    
    let record_name = format!("{}.{}", test_prefix, test_domain);
    
    let duty = Duty {
        id: Some(4),
        name: "test-route53-cname".to_string(),
        duty_type: "Route53Record".to_string(),
        backend: "aws".to_string(),
        roster_selector: json!({"traits": ["cloud-provider", "aws"]}),
        spec: json!({
            "hosted_zone_id": hosted_zone_id,
            "name": record_name,
            "record_type": "CNAME",
            "value": "example.com",
            "ttl": 300
        }),
        status: Some(json!({})),
        metadata: Some(json!({})),
        created_at: None,
        updated_at: None,
    };
    
    module.validate(&roster, &duty).await?;
    
    let result = module.apply(&roster, &duty).await?;
    assert_eq!(result["phase"], "deployed");
    
    module.destroy(&roster, &duty).await?;
    
    Ok(())
}

#[tokio::test]
#[ignore = "Integration test - requires AWS credentials"]
async fn test_iam_user_with_access_keys() -> Result<()> {
    let state = init_state_manager().await?;
    let module = IAMUserModule::new(state);
    let roster = create_aws_roster();
    let test_prefix = get_test_prefix();
    let user_name = format!("{}-user", test_prefix);
    
    let duty = Duty {
        id: Some(5),
        name: "test-iam-user".to_string(),
        duty_type: "IAMUser".to_string(),
        backend: "aws".to_string(),
        roster_selector: json!({"traits": ["cloud-provider", "aws"]}),
        spec: json!({
            "user_name": user_name,
            "create_access_key": true,
            "inline_policies": {
                "S3ReadOnly": {
                    "Version": "2012-10-17",
                    "Statement": [{
                        "Effect": "Allow",
                        "Action": "s3:GetObject",
                        "Resource": "*"
                    }]
                }
            }
        }),
        status: Some(json!({})),
        metadata: Some(json!({})),
        created_at: None,
        updated_at: None,
    };
    
    module.validate(&roster, &duty).await?;
    
    let result = module.apply(&roster, &duty).await?;
    assert_eq!(result["phase"], "deployed");
    assert!(result["outputs"]["access_key_id"].is_string());
    assert!(result["outputs"]["secret_access_key_ref"].is_string());
    
    module.destroy(&roster, &duty).await?;
    
    Ok(())
}
