# Project Architecture

This document serves as an overall guide to the structure, dependencies, and interactions within the project. It is intended to assist new contributors and maintainers in understanding the purpose and relationships between the various modules.

## Overview

The project is organized around algorithmic trading and consists of several interrelated components. Each component is designed for a specific purpose within the trading ecosystem:

- **Algo Trading**: The main hub coordinating the trading activities.
- **Backtester**: Provides historical data simulation to test trading strategies.
- **Broker Shims**: Contains adapters for interacting with different broker APIs, such as FXCM and Simbroker.
- **Configurator**: Manages configuration settings and environment parameters.
- **Data Downloaders**: Responsible for fetching market data from various sources (e.g., FXCM, IEX, Poloniex).
- **Logger**: Implements logging functionality for diagnostics and debugging.
- **Market Monitor (MM & mm-react)**: Includes modules for real-time monitoring and user interfaces.
- **Optimizer**: Optimizes trading strategies by evaluating performance over historical data.
- **Private, Spawner, Tick Parser, Tick Writer, Util**: Additional modules that support specific tasks such as data parsing, order execution, and utility functions.

## Dependencies & Requirements

- **Rust**: Used in components like Backtester, Broker Shims, Configurator, Logger, Optimizer, and others.
- **JavaScript/Node.js**: Applied in Data Downloaders, Market Monitor (mm-react), and other scripting components.
- **Shell Scripts**: Various scripts for builds, tests, and deployments.
- **Other Tools**: Java components may be used in the broker shims integration (e.g., FXCM Java Bridge).

## Build and Deployment

- Each Rust component has its own `Cargo.toml` and may be managed as individual crates.
- Common build scripts (e.g., Makefile, shell scripts) are located in the related directories.
- Consider containerization (e.g., Docker) for a consistent development environment.
- Automated testing and CI pipelines are recommended for a streamlined deployment process.

## Testing & Quality Assurance

- **Unit and Integration Tests**: Embedded within Rust and JavaScript modules.
- **Linting and Formatting**: Use tools such as `rustfmt` for Rust and ESLint/Prettier for JavaScript.
- **Continuous Integration**: Setting up CI pipelines will help ensure code quality with every commit.

## Documentation & Onboarding

- Comprehensive README files are provided in key directories.
- This document offers an overview of the project architecture.
- Onboarding guidelines:
  - Review individual README files for module-specific details.
  - Use this document as a starting point to understand overall dependencies and interactions.
  - Follow the provided setup and build instructions to get started quickly.

## Conclusion

This architecture document is a living guide meant to evolve with the project. It provides an initial framework for understanding how the different pieces collaborate and can serve as a reference for further enhancements and documentation efforts.
