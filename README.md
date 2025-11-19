# g8r

![g8r](./assets/g8r_.png)

## Overview
g8r (pronounced "gator") is a powerful configuration management and event-driven automation engine designed to streamline and automate the management of infrastructure and services. It enables dynamic responses to infrastructure events, facilitating seamless and automated operations.

### Reproducible Infrastructure

Inspired by Nix, G8R treats infrastructure as **reproducible, declarative state**. Your Nickel configuration files are the single source of truth - at any time, you can use them to:

- **Recreate your entire infrastructure from scratch** - Deploy to a new AWS account and get identical infrastructure
- **Restore after disasters** - Your Git repository contains everything needed to rebuild
- **Clone environments** - Copy production to staging with predictable results
- **Time-travel your infrastructure** - Git checkout any commit and deploy that exact state

The combination of:
- **Nickel DSL** (pure, functional configuration)
- **PostgreSQL state tracking** (what's deployed)
- **Git version control** (history and rollback)
- **Trait-based modules** (consistent behavior across platforms)

...ensures that `g8r deploy --all` produces the same infrastructure every time, regardless of when or where you run it. No drift, no surprises, just reproducible infrastructure.

## Quick Start

### Prerequisites
- Nix with flakes enabled
- Podman (for PostgreSQL container)
- AWS credentials configured
- GitHub token (for repository/secrets management)

### Development Setup

```bash
# Enter the Nix development environment (provides g8r-* commands)
nix develop

# Start PostgreSQL (creates and starts container if needed)
g8r-db-setup

# Start the API server (automatically loads .env)
g8r-server

# In another terminal, test the server
curl http://localhost:8080/health
```

The `nix develop` shell provides:
- `g8r-db-start` - Start/create PostgreSQL container only
- `g8r-db-setup` - Start PostgreSQL + apply database schema
- `g8r-db-stop` - Stop PostgreSQL container  
- `g8r-server` - Run API server (loads .env automatically)
- Standard tools: cargo, rust-analyzer, psql, aws, etc.

### Database Management

```bash
# The g8r-db-setup command handles everything:
# - Creates postgres:16-alpine container if it doesn't exist
# - Starts container if it's stopped
# - Configures database: g8r_state with user g8r
# - Applies database schema from init.sql
# - Exposes port 5432 for local connections

# Stop the database when done
g8r-db-stop

# Connect directly with psql
psql postgresql://g8r:g8r_dev_password@localhost:5432/g8r_state
```

### Server Details

When you run `g8r-server`, it:
- Loads environment variables from `.env` automatically
- Initializes OpenTelemetry tracing (configured via `OTEL_EXPORTER`)
- Connects to PostgreSQL using DATABASE_URL
- Starts reconciliation loops for git-tracked stacks
- Serves REST API on port 8080

Server logs show:
```
INFO g8r: Starting G8R API server
INFO g8r::stack::manager: Found 1 stacks to manage
INFO g8r::api::server: API server listening on http://0.0.0.0:8080
```

## Architecture

G8R uses:
- **Nickel DSL** for configuration (with Nix-like merge semantics)
- **PostgreSQL** for state management
- **Rust trait system** for pluggable AWS modules
- **AWS SDK** for infrastructure management (S3, CloudFront, ACM, IAM, Route53)
- **GitHub API** for repository and secrets management

### Key Components

- `src/nickel/` - Nickel configuration evaluator
- `src/db/` - PostgreSQL state management
- `src/aws/` - AWS service modules (S3, CloudFront, ACM, IAM, Route53)
- `src/github/` - GitHub API integration
- `src/stack/` - Stack orchestrators (currently: static website)
- `src/cli/` - CLI interface

## Environment Configuration

G8R uses a `.env` file for environment-specific configuration:

```bash
# Database
DATABASE_URL=postgresql://g8r:g8r_dev_password@localhost:5432/g8r_state

# GitHub authentication (for private repository access)
GITHUB_TOKEN=github_pat_xxxxx

# AWS configuration (optional, uses AWS CLI config if not set)
AWS_REGION=us-east-2
AWS_ACCESS_KEY_ID=AKIA...
AWS_SECRET_ACCESS_KEY=...
# AWS_PROFILE=your-profile

# Logging
RUST_LOG=info

# OpenTelemetry configuration
OTEL_EXPORTER=file            # Options: stdout, file, jaeger, otlp
OTEL_SERVICE_NAME=g8r
LOG_FILE=g8r.log              # Only used when OTEL_EXPORTER=file
```

### Telemetry Options

- **file**: Logs to file (default: `g8r.log`, configurable via `LOG_FILE`)
- **stdout**: Logs to console (good for interactive debugging)
- **jaeger**: Exports to Jaeger (requires Jaeger running on localhost:6831)
- **otlp**: Exports to OTLP collector (requires OTLP_ENDPOINT configured)

The trace level is controlled by `RUST_LOG` (e.g., `debug`, `info`, `warn`, `error`).

To tail logs when using file output:
```bash
tail -f g8r.log
```

## Configuration

### Project Structure

G8R uses a **stack-based architecture** with rosters, duties, and git-tracked configuration:

```
your-iac-repo/           # Your private infrastructure repo (tracked as a Stack)
├── base.ncl            # Base configuration
├── dev.ncl             # Development environment
├── staging.ncl         # Staging environment
├── prod.ncl            # Production environment
└── .env                # Local overrides (not tracked)

g8r/                    # This repository (the engine)
└── src/               # G8R source code
```

### Stack Configuration (Nickel)

Stacks are git repositories containing Nickel configurations that define rosters and duties:

```nickel
# base.ncl - Shared configuration
{
  roster = {
    name = "production-aws",
    roster_type = "aws-account",
    traits = ["cloud-provider", "aws", "us-east-1"],
    connection = {
      region = "us-east-1",
    },
  },
  
  duties = {
    "website-hosting" = {
      duty_type = "s3-static-site",
      backend = "aws",
      roster_selector = {
        all = ["cloud-provider", "aws"],
      },
      spec = {
        bucket_name = "my-production-site",
      },
    },
  },
}
```

```nickel
# dev.ncl - Environment override
let base = import "base.ncl" in

base & {
  roster = {
    name = "dev-aws",
    connection = {
      region = "us-east-2",
    },
  },
  
  duties = {
    "website-hosting" = {
      spec = {
        bucket_name = "dev-site",
      },
    },
  },
}
```

## Usage

```bash
# Deploy all clients
g8r deploy --all

# Deploy specific client
g8r deploy --client acmecorp

# Deploy specific environment
g8r deploy --client acmecorp --environment staging

# List all clients
g8r list --clients

# Plan changes (not yet implemented)
g8r plan --client acmecorp

# Destroy resources (requires confirmation)
g8r destroy --client acmecorp --environment staging --confirm

# Start API server
g8r serve --host 0.0.0.0 --port 8080
```

### API Server

Start the API server to manage rosters, duties, and stacks:

```bash
# Start the server (loads .env automatically)
cargo run -- serve --host 0.0.0.0 --port 8080

# In another terminal, query the API
curl http://localhost:8080/health
curl http://localhost:8080/api/v1/rosters
curl http://localhost:8080/api/v1/duties
curl http://localhost:8080/api/v1/stacks

# Trigger manual stack sync
curl -X POST http://localhost:8080/api/v1/stacks/w4b-iac-dev/sync
```

See [openapi.yaml](openapi.yaml) for complete API documentation.

### Development Scripts

G8R includes Nushell scripts in `scripts/`:

```bash
# Run tests in watch mode
nu scripts/test-watch.nu

# Run all tests
nu scripts/test-run.nu

# Test setup utilities
nu scripts/test-setup.nu
```

## Roadmap

### 🎯 Current Milestone: Phase 1 - Foundation (v0.1.0)

**Goal:** Replace Pulumi for static website deployments with a reproducible, trait-based Rust implementation.

**Status:** Core infrastructure complete, working on stability and testing.

#### Completed in Phase 1:
- ✅ Trait-based AWS service modules (S3, CloudFront, ACM, IAM, Route53)
- ✅ PostgreSQL state tracking and deployment history
- ✅ Multi-client, multi-environment support
- ✅ GitHub integration (repositories, secrets)
- ✅ REST API server for monitoring
- ✅ Nickel DSL evaluation framework
- ✅ CLI interface with deploy/list/serve commands
- ✅ Containerized deployment with Podman

#### Next Steps (v0.1.x):
- 🚧 Integration testing with real AWS infrastructure
- 🚧 Drift detection and reconciliation
- 🚧 Plan/preview functionality
- 🚧 Resource destruction with safety checks
- 🚧 Enhanced error handling and recovery
- 🚧 Comprehensive unit test coverage

### Phase 2 - Platform Expansion (v0.2.0)
- Multi-cloud support (Azure, GCP)
- Additional stack types (container apps, databases)
- Terraform state import
- Dependency graph visualization

### Phase 3 - Distributed Architecture (v0.3.0)
- Controller/worker architecture for K8s
- Event-driven reconciliation loops
- Distributed hash ring coordination
- Horizontal scaling support

### Phase 4 - WebAssembly Ecosystem (v0.4.0)
- WASM module system for custom integrations
- Platform abstraction layer
- Community plugin marketplace
- Non-K8s deployment targets (IoT, edge)

For detailed architectural vision, see [FUTURE.md](FUTURE.md).

## Contributing

Contributions welcome! Please ensure:
- Code follows Rust idioms
- Tests are included for new functionality
- Nickel configurations are validated
- Documentation is updated

## Licensing 

g8r is available for individual use under the following terms:

Usage: Individuals are granted a non-exclusive, non-transferable, revocable license to use g8r for personal, non-commercial purposes.

Restrictions:

Commercial use of g8r is strictly prohibited under this license. Any use of g8r in a commercial environment or for commercial purposes requires a separate commercial license.
Redistribution, modification, sublicensing, and derivative works are not permitted unless expressly authorized by a separate agreement.
Disclaimer: This software is provided "as is", without warranty of any kind, express or implied.

Copyright: © 2023 Brian Logan. All rights reserved.

## Support

For issues, questions, or contributions, please open an issue on the repository.
