let base = import "base.ncl" in

base & {
  roster = {
    name = "prod-aws",
    connection = {
      region = "us-east-1",
    },
  },
  
  duties = {
    "site-bucket" = {
      spec = {
        bucket_name = "prod-g8r-test",
      },
    },
  },
}
