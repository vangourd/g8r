let base = import "base.ncl" in

base & {
  roster = {
    name = "staging-aws",
    connection = {
      region = "us-east-1",
    },
  },
  
  duties = {
    "site-bucket" = {
      spec = {
        bucket_name = "staging-g8r-test",
      },
    },
  },
}
