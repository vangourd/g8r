{
  const = {
    environment = "dev",
    domain = "example.dev",
    bucket_name = "dev-example",
  },
  
  roster = {
    name = "dev-aws",
    type = "aws-account",
    region = "us-east-1",
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
        domain = "example.test",
        alternate_domains = [],
      },
      outputs = ["arn", "validation_record"],
    },
    
    "AWS::Route53Record::cert-validation" = {
      inputs = {
        "$AWS::ACMCertificate::site-cert::validation_record" = "validation_record",
        record_type = "CNAME",
        ttl = 300,
        name = "$validation_record.name",
        value = "$validation_record.value",
      },
    },
    
    "AWS::CloudFrontDistribution::site-cdn" = {
      inputs = {
        "$AWS::S3Bucket::site-bucket::website_endpoint" = "origin",
        "$AWS::ACMCertificate::site-cert::arn" = "certificate",
        aliases = ["$const.domain"],
      },
      outputs = ["distribution_domain", "distribution_id"],
    },
    
    "AWS::Route53Record::site-dns" = {
      inputs = {
        "$AWS::CloudFrontDistribution::site-cdn::distribution_domain" = "alias",
        record_name = "$const.domain",
        record_type = "A",
      },
    },
    
    "AWS::IAMUser::cicd-user" = {
      inputs = {
        "$AWS::S3Bucket::site-bucket::bucket_name" = "s3_bucket",
        user_name = "example-dev-cicd",
        policy = {
          actions = ["s3:PutObject", "s3:GetObject", "s3:DeleteObject", "s3:ListBucket"],
        },
      },
      outputs = ["arn", "access_key_id", "secret_access_key"],
    },
  },
}
