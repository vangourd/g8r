let base = import "base.ncl" in

base & {
  const = {
    environment = "dev",
    domain = "example.dev",
    bucket_name = "dev-example",
  },
  
  roster = {
    name = "dev-aws",
  },
  
  resources = {
    "AWS::ACMCertificate::site-cert" = {
      inputs | force = {
        domain = "example.test",
        alternate_domains = [],
      },
    },
  },
}
