let base = import "base.ncl" in

base & {
  rosters | force = [{
    name = "dev-aws",
    roster_type = "aws-account",
    traits = ["cloud-provider", "aws"],
    connection = {
      region = "us-east-1",
    },
    auth = {},
  }],
  
  duties = {
    "site-bucket" = {
      spec = {
        bucket_name = "dev-example",
      },
    },
    
    "site-cert" = {
      name = "site-cert",
      duty_type = "ACMCertificate",
      backend = "aws",
      roster_selector = {
        traits = ["aws"]
      },
      spec = {
        domain_name = "example.test",
        subject_alternative_names = [],
      },
      metadata = {},
    },
    
    "cert-validation-record" = 
      let cert_validation = 
        if std.record.has_field "site-cert" runtime.duties then
          let cert_duty = runtime.duties."site-cert" in
          if std.record.has_field "outputs" cert_duty then
            let outputs = cert_duty.outputs in
            if std.record.has_field "validation_records" outputs then
              if std.array.length outputs.validation_records > 0 then
                std.array.first outputs.validation_records
              else { name = "", value = "" }
            else { name = "", value = "" }
          else { name = "", value = "" }
        else { name = "", value = "" }
      in
      {
        name = "cert-validation-record",
        duty_type = "Route53Record",
        backend = "aws",
        roster_selector = {
          traits = ["aws"]
        },
        spec = {
          hosted_zone_id = "",
          name = cert_validation.name,
          record_type = "CNAME",
          value = cert_validation.value,
          ttl = 300,
        },
        depends_on = ["site-cert"],
        metadata = {},
      },
    
    "site-cdn" = 
      let bucket_endpoint = 
        if std.record.has_field "site-bucket" runtime.duties then
          let bucket_duty = runtime.duties."site-bucket" in
          if std.record.has_field "outputs" bucket_duty then
            if std.record.has_field "website_endpoint" bucket_duty.outputs then
              bucket_duty.outputs.website_endpoint
            else ""
          else ""
        else ""
      in
      let cert_arn = 
        if std.record.has_field "site-cert" runtime.duties then
          let cert_duty = runtime.duties."site-cert" in
          if std.record.has_field "outputs" cert_duty then
            if std.record.has_field "arn" cert_duty.outputs then
              cert_duty.outputs.arn
            else ""
          else ""
        else ""
      in
      {
        name = "site-cdn",
        duty_type = "CloudFrontDistribution",
        backend = "aws",
        roster_selector = {
          traits = ["aws"]
        },
        spec = {
          origin = {
            domain_name = bucket_endpoint,
          },
          aliases = ["example.dev"],
          certificate_arn = cert_arn,
        },
        depends_on = ["site-bucket", "site-cert"],
        metadata = {},
      },
    
    "site-dns" = 
      let cdn_domain = 
        if std.record.has_field "site-cdn" runtime.duties then
          let cdn_duty = runtime.duties."site-cdn" in
          if std.record.has_field "outputs" cdn_duty then
            if std.record.has_field "distribution_domain" cdn_duty.outputs then
              cdn_duty.outputs.distribution_domain
            else ""
          else ""
        else ""
      in
      {
        name = "site-dns",
        duty_type = "Route53Record",
        backend = "aws",
        roster_selector = {
          traits = ["aws"]
        },
        spec = {
          hosted_zone_id = "",
          name = "example.dev",
          record_type = "A",
          alias = {
            dns_name = cdn_domain,
            hosted_zone_id = "",
          },
        },
        depends_on = ["site-cdn"],
        metadata = {},
      },
    
    "site-cicd-user" = {
      name = "site-cicd-user",
      duty_type = "IAMUser",
      backend = "aws",
      roster_selector = {
        traits = ["aws"]
      },
      spec = {
        user_name = "example-dev-cicd",
        policy = {
          Version = "2012-10-17",
          Statement = [{
            Effect = "Allow",
            Action = ["s3:PutObject", "s3:GetObject", "s3:DeleteObject", "s3:ListBucket"],
            Resource = [
              "arn:aws:s3:::dev-example",
              "arn:aws:s3:::dev-example/*"
            ],
          }],
        },
      },
      depends_on = ["site-bucket"],
      metadata = {},
    },
  },
}
