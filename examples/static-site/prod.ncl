let base = import "base.ncl" in

base & {
  const = {
    environment = "prod",
    domain = "example.com",
    bucket_name = "prod-g8r-test",
  },
  
  roster = {
    name = "prod-aws",
  },
}
