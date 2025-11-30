{
  const = {
    environment | force = "changeme",
    domain | force = "example.com", 
    bucket_name | force = "changeme-bucket",
  },
  
  roster = {
    name | force = "base-aws",
    type = "aws-account",
    region | default = "us-east-1",
  },
  
  resources = {
    "AWS::S3Bucket::site-bucket" = {
      inputs = {
        bucket_name = "$const.bucket_name",
        website = {
          index = "index.html",
          error = "404.html",
        },
      },
      outputs = ["arn", "website_endpoint", "bucket_name"],
    },
    
    "AWS::ACMCertificate::site-cert" = {
      inputs = {
        domain = "$const.domain",
        alternate_domains | default = [],
      },
      outputs = ["arn", "validation_record"],
    },
    
    "AWS::Route53Record::cert-validation" = {
      inputs = {
        validation_record = "$AWS::ACMCertificate::site-cert::validation_record",
        record_type = "CNAME",
        ttl = 300,
        name = "$validation_record.name",
        value = "$validation_record.value",
      },
    },
    
    "AWS::CloudFrontDistribution::site-cdn" = {
      inputs = {
        origin = "$AWS::S3Bucket::site-bucket::website_endpoint",
        certificate = "$AWS::ACMCertificate::site-cert::arn",
        aliases = ["$const.domain"],
      },
      outputs = ["distribution_domain", "distribution_id"],
    },
    
    "AWS::Route53Record::site-dns" = {
      inputs = {
        alias = "$AWS::CloudFrontDistribution::site-cdn::distribution_domain",
        record_name = "$const.domain",
        record_type = "A",
      },
    },
    
    "AWS::IAMUser::cicd-user" = {
      inputs = {
        s3_bucket = "$AWS::S3Bucket::site-bucket::bucket_name",
        user_name = "$const.environment-cicd",
        policy = {
          actions = ["s3:PutObject", "s3:GetObject", "s3:DeleteObject", "s3:ListBucket"],
        },
      },
      outputs = ["arn", "access_key_id", "secret_access_key"],
    },
  },
}
