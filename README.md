
# g8r

![g8r](./assets/g8r_.png)

## Overview
g8r (pronounced "gator") is a powerful configuration management and event-driven automation engine designed to streamline and automate the management of infrastructure and services. It enables dynamic responses to infrastructure events, facilitating seamless and automated operations.

## Current Architecture Plans
```mermaid
classDiagram
    class Main {
        run()
    }
    class Config {
        <<crate>>
        +String path
        +String settings
        --parseConfig()
    }
    class Repo {
        <<crate>>
        +String repositoryName
        +String repositoryURL
        --manageRepository()
    }
    class Roster {
        <<crate>>
        +HashMap String, Vec, String
        get_duties() for hostname
    }
    class Duty {
        <<crate>>
        +String name
        +List~Task~
        --trackOrchestration()
    }
    class Task {
        <<trait>>
        +performExecution()
        +selectModule(String moduleName)
    }
    class Echo implements Task {
        <<crate>>
        +String echoContext
        --performEcho()
        --selectEchoModule(String moduleName)
    }

    Main -- Config : uses
    Main -- Repo : uses
    Main -- Roster : uses
    Roster "1" -- "*" Duty : contains
    Duty "1" -- "*" Echo : holds
    Echo ..|> Task : implements
```

## Features
None currently it's purely theoretical

## Installation
[Instructions on how to install g8r]

## Quick Start
[Guide on how to quickly get started with g8r]

## Contributing

## Licensing 
g8r is available for individual use under the following terms:

Usage: Individuals are granted a non-exclusive, non-transferable, revocable license to use g8r for personal, non-commercial purposes.

Restrictions:

Commercial use of g8r is strictly prohibited under this license. Any use of g8r in a commercial environment or for commercial purposes requires a separate commercial license.
Redistribution, modification, sublicensing, and derivative works are not permitted unless expressly authorized by a separate agreement.
Disclaimer: This software is provided "as is", without warranty of any kind, express or implied.

Copyright: © 2023 Brian Logan. All rights reserved.

## Community

## Support