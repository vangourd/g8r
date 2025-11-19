use anyhow::Result;
use g8r::controller::Controller;
use g8r::db::StateManager;
use g8r::modules::aws::s3_bucket::S3BucketModule;
use g8r::modules::aws::route53_record::Route53RecordModule;
use std::env;
use std::io::Write;
use std::sync::Arc;
use tempfile::NamedTempFile;

async fn init_test_controller() -> Result<Controller> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://g8r:g8r_dev_password@localhost:5432/g8r_state".to_string());
    
    let state = StateManager::new(&database_url).await?;
    
    let mut controller = Controller::new(state.clone());
    
    controller.register_module(Arc::new(S3BucketModule::new(state.clone())));
    controller.register_module(Arc::new(Route53RecordModule::new(state.clone())));
    
    Ok(controller)
}

#[tokio::test]
#[ignore = "Integration test - requires database and nickel CLI"]
async fn test_runtime_injection_with_dependencies() -> Result<()> {
    let controller = init_test_controller().await?;
    
    let hosted_zone_id = env::var("TEST_HOSTED_ZONE_ID")
        .expect("TEST_HOSTED_ZONE_ID must be set for Route53 tests");
    let test_domain = env::var("TEST_DOMAIN")
        .expect("TEST_DOMAIN must be set for Route53 tests");
    
    let mut config_file = NamedTempFile::new()?;
    writeln!(
        config_file,
        r#"
{{
  rosters = {{
    test-aws = {{
      name = "test-aws",
      roster_type = "aws-account",
      traits = ["cloud-provider", "aws"],
      connection = {{ region = "us-east-2" }},
      auth = {{}},
    }}
  }},
  duties = {{
    test-bucket = {{
      duty_type = "S3Bucket",
      backend = "aws",
      roster_selector = {{ traits = ["cloud-provider", "aws"] }},
      spec = {{
        bucket_name = "g8r-runtime-test-{}",
      }},
    }},
    test-dns = {{
      duty_type = "Route53Record",
      backend = "aws",
      depends_on = ["test-bucket"],
      roster_selector = {{ traits = ["cloud-provider", "aws"] }},
      spec = 
        let bucket_name = 
          if std.record.has_field "test-bucket" runtime.duties then
            runtime.duties."test-bucket".outputs.bucket_name
          else
            "placeholder-will-be-replaced"
        in
        {{
          hosted_zone_id = "{}",
          name = "runtime-test-{}.{}",
          record_type = "CNAME",
          value = bucket_name ++ ".s3-website.us-east-2.amazonaws.com",
          ttl = 300,
        }},
    }}
  }}
}}
"#,
        chrono::Utc::now().timestamp(),
        hosted_zone_id,
        chrono::Utc::now().timestamp(),
        test_domain
    )?;
    config_file.flush()?;
    
    let result = controller.reconcile_from_nickel(config_file.path().to_str().unwrap()).await;
    
    match result {
        Ok(_) => {
            println!("✅ Runtime injection test passed");
            Ok(())
        }
        Err(e) => {
            eprintln!("ERROR: {:#?}", e);
            if e.to_string().contains("unbound identifier `runtime`") {
                panic!("❌ Runtime injection failed: runtime variable not injected for Batch 0");
            } else if e.to_string().contains("AccessDenied") || e.to_string().contains("no identity-based policy") {
                println!("⚠️  Test skipped: AWS permissions not configured");
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

#[tokio::test]
#[ignore = "Integration test - requires database and nickel CLI"]
async fn test_batch_zero_without_runtime_refs() -> Result<()> {
    let controller = init_test_controller().await?;
    
    let mut config_file = NamedTempFile::new()?;
    writeln!(
        config_file,
        r#"
{{
  rosters = {{
    test-aws = {{
      name = "test-aws-batch0",
      roster_type = "aws-account",
      traits = ["cloud-provider", "aws"],
      connection = {{ region = "us-east-2" }},
      auth = {{}},
    }}
  }},
  duties = {{
    bucket1 = {{
      duty_type = "S3Bucket",
      backend = "aws",
      roster_selector = {{ traits = ["cloud-provider", "aws"] }},
      spec = {{
        bucket_name = "g8r-batch0-test1-{}",
      }},
    }},
    bucket2 = {{
      duty_type = "S3Bucket",
      backend = "aws",
      roster_selector = {{ traits = ["cloud-provider", "aws"] }},
      spec = {{
        bucket_name = "g8r-batch0-test2-{}",
      }},
    }},
  }}
}}
"#,
        chrono::Utc::now().timestamp(),
        chrono::Utc::now().timestamp() + 1
    )?;
    config_file.flush()?;
    
    let result = controller.reconcile_from_nickel(config_file.path().to_str().unwrap()).await;
    
    match result {
        Ok(_) => {
            println!("✅ Batch 0 (no dependencies) executed successfully");
            Ok(())
        }
        Err(e) => {
            if e.to_string().contains("AccessDenied") || e.to_string().contains("no identity-based policy") {
                println!("⚠️  Test skipped: AWS permissions not configured");
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

#[tokio::test]
#[ignore = "Integration test - requires database and nickel CLI"]
async fn test_multi_batch_dependency_chain() -> Result<()> {
    let controller = init_test_controller().await?;
    
    let hosted_zone_id = env::var("TEST_HOSTED_ZONE_ID")
        .expect("TEST_HOSTED_ZONE_ID must be set for Route53 tests");
    let test_domain = env::var("TEST_DOMAIN")
        .expect("TEST_DOMAIN must be set for Route53 tests");
    
    let mut config_file = NamedTempFile::new()?;
    writeln!(
        config_file,
        r#"
{{
  rosters = {{
    test-aws = {{
      name = "test-aws-chain",
      roster_type = "aws-account",
      traits = ["cloud-provider", "aws"],
      connection = {{ region = "us-east-2" }},
      auth = {{}},
    }}
  }},
  duties = {{
    bucket = {{
      duty_type = "S3Bucket",
      backend = "aws",
      roster_selector = {{ traits = ["cloud-provider", "aws"] }},
      spec = {{
        bucket_name = "g8r-chain-test-{}",
      }},
    }},
    dns1 = {{
      duty_type = "Route53Record",
      backend = "aws",
      depends_on = ["bucket"],
      roster_selector = {{ traits = ["cloud-provider", "aws"] }},
      spec = 
        let bucket_arn = 
          if std.record.has_field "bucket" runtime.duties then
            runtime.duties.bucket.outputs.arn
          else
            "placeholder-arn"
        in
        {{
          hosted_zone_id = "{}",
          name = "chain-test1-{}.{}",
          record_type = "TXT",
          value = "bucket-arn=" ++ bucket_arn,
          ttl = 300,
        }},
    }},
    dns2 = {{
      duty_type = "Route53Record",
      backend = "aws",
      depends_on = ["dns1"],
      roster_selector = {{ traits = ["cloud-provider", "aws"] }},
      spec = 
        let record_id = 
          if std.record.has_field "dns1" runtime.duties then
            runtime.duties.dns1.outputs.record_id
          else
            "placeholder-record-id"
        in
        {{
          hosted_zone_id = "{}",
          name = "chain-test2-{}.{}",
          record_type = "TXT",
          value = "dns1-record=" ++ record_id,
          ttl = 300,
        }},
    }},
  }}
}}
"#,
        chrono::Utc::now().timestamp(),
        hosted_zone_id,
        chrono::Utc::now().timestamp(),
        test_domain,
        hosted_zone_id,
        chrono::Utc::now().timestamp(),
        test_domain
    )?;
    config_file.flush()?;
    
    let result = controller.reconcile_from_nickel(config_file.path().to_str().unwrap()).await;
    
    match result {
        Ok(_) => {
            println!("✅ Multi-batch dependency chain executed successfully");
            Ok(())
        }
        Err(e) => {
            if e.to_string().contains("unbound identifier `runtime`") {
                panic!("❌ Runtime injection failed in dependency chain");
            } else if e.to_string().contains("AccessDenied") || e.to_string().contains("no identity-based policy") {
                println!("⚠️  Test skipped: AWS permissions not configured");
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}
