{
  rosters = [{
    name | default = "base-roster",
    roster_type = "aws-account",
    traits = ["cloud-provider", "aws"],
    connection = {
      region | default = "us-east-1",
    },
    auth = {},
  }],
  
  duties = {
    "site-bucket" = {
      name = "site-bucket",
      duty_type = "S3Bucket",
      backend = "aws",
      roster_selector = {
        traits = ["aws"]
      },
      spec = {
        bucket_name | default = "changeme-bucket",
        website_config = {
          index_document = "index.html",
          error_document = "404.html",
        },
      },
      metadata = {},
    },
  },
}
