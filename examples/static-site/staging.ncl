let base = import "base.ncl" in

base & {
  const = {
    environment = "staging",
    domain = "staging.example.com",
    bucket_name = "staging-g8r-test",
  },
  
  roster = {
    name = "staging-aws",
  },
}
