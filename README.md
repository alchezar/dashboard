# Project: Full-Stack Proxmox Management Platform in Rust & React

### Objective

Build a complete WHMCS replacement: a Rust API orchestrating Proxmox VE, a React dashboard for server management, and a migration utility to transfer legacy data.

### Core Deliverables

* **WHMCS Testbed:** Local legacy system simulation for migration testing
* **Proxmox VE Testbed:** Local virtualization environment for API integration
* **Rust API:** Server lifecycle orchestration with Proxmox integration
* **React Dashboard:** Modern web interface for server management
* **Migration Utility:** Command-line tool for WHMCS-to-PostgreSQL data transfer

### Environment Setup

* **WHMCS Testbed:** Local instance with sample users, server products, and complex **Configurable Options** (`RAM`, `CPU`) and **Custom Fields** (`OS templates`, `Datacenter locations`) for migration testing
* **Proxmox VE Testbed:** Running node with cloneable VM templates (Cloud-Init Ubuntu) accessible from the Rust API

### Rust API (Axum + PostgreSQL)

* **Architecture:** Axum web framework, PostgreSQL persistence (SQLx/Diesel), structured logging (tracing)
* **Core Features:** JWT authentication, async server provisioning (202 Accepted), full lifecycle endpoints
* **Proxmox Integration:** Direct API integration for VM cloning, lifecycle management, and resource orchestration

### React Dashboard (Vite SPA)

* **Stack:** React with Vite, modern UI component library
* **Features:** JWT authentication, server listing with specs/status, live status updates via polling, direct server lifecycle actions

### Migration Utility (Rust CLI)

* **Direct Database Extraction:** The utility will connect directly to the WHMCS MySQL database to extract user and server data.
* **Core Transformation Logic:** The script's primary purpose is to prove it can migrate active users and their servers. The most critical task is to correctly extract and transform data from WHMCS's **Configurable Options and Custom Fields** into the new, structured PostgreSQL schema.
* **Essential Safeguards:** To prove the migration strategy is viable and safe, the utility must include:
* **Idempotency:** The script must be safe to run multiple times without creating duplicate data.
* **A `--dry-run` flag:** This will print a summary of actions that would be taken, allowing for validation without committing any changes.

### Full-Stack Quality and Validation

To validate the success of this build, a comprehensive test and benchmark suite covering both the frontend and backend is required.

* **Backend Testing:**
* **Unit Tests:** Primarily for the migration's transformation logic, ensuring the mapping from WHMCS fields is correct.
* **Integration Tests:** To verify the core API endpoints, including authentication, error handling, and successful orchestration of a server creation request.
* **Frontend Testing:**
* **Component Tests:** Basic tests (e.g., using Vitest/React Testing Library) must be included to verify the behavior of critical UI components like the login form and the server dashboard display.
* **Professional Performance Benchmarking:**
* **API Performance:** JWT authentication throughput, database query speeds, async task processing latency, and memory usage profiling
* **Migration Performance:** WHMCS transformation speed, bulk insert rates, memory efficiency, and idempotency validation
* **Frontend Performance:** React component render times, API response handling, and bundle size optimization
* **End-to-End Validation:** Complete workflow timing, concurrent user load testing, and connection pool performance

### Extended Technology & Protocol Stack

This stack includes the technologies explicitly mentioned and those implicitly required to successfully complete the project.

**Core Application & Language**

* **Rust:** The primary programming language for the API and migration utility.
* **Tokio:** The asynchronous runtime powering the `axum` web framework.
* **Axum:** The web framework for building the core REST API.
* **SQLx:** The asynchronous SQL toolkit for Rust, used for interacting with PostgreSQL.
* **Serde:** The framework for serializing and deserializing Rust data structures, primarily for JSON API payloads.
* **Clap:** A command-line argument parser for building the standalone migration utility.
* **Tracing:** A framework for instrumenting Rust programs to collect structured, event-based diagnostic information.

**Database Systems**

* **PostgreSQL:** The target database for the new, modern application.
* **MySQL:** The source database used by the legacy WHMCS instance.
* **SQL:** Proficiency in both PostgreSQL and MySQL dialects is required.

**Web & API Technologies**

* **RESTful API Design:** Principles for creating clean, predictable web APIs.
* **HTTP:** Core understanding of methods (GET, POST), status codes (e.g., 200 OK, 202 Accepted, 401 Unauthorized), and headers.
* **JSON (JavaScript Object Notation):** The data format for API communication.
* **JWT (JSON Web Tokens):** The standard for stateless API authentication. Crates like `jsonwebtoken` would be used.

**Legacy System & Environment**

* **WHMCS:** The legacy PHP-based billing and automation platform.
* **PHP:** Basic understanding is helpful for navigating the WHMCS environment.
* **LAMP Stack (Linux, Apache, MySQL, PHP):** The typical environment for running WHMCS.

**Development, Tooling & Design Patterns**

* **Docker & Docker Compose:** For creating a reproducible local development environment that includes the WHMCS testbed, PostgreSQL, and the new Rust API.
* **Cargo:** Rust's build system and package manager.
* **Git & GitHub/GitLab:** For version control.
* **`sqlx-cli`:** A command-line utility for managing database migrations (creating, applying, reverting).
* **Mocking:** Using traits and dependency injection to mock external services (`ProxmoxClient`) for testing. Crates like `wiremock` are common.
* **Background Worker Pattern:** A design pattern for offloading long-running tasks (like server provisioning) from the main API request-response cycle.

---

### Required Skill Set

This translates the technology stack into the practical skills needed to execute the project.

**1. Rust Backend Development**

* **Web API Construction:** Proficiency in building robust, asynchronous REST APIs using `axum`, including routing, state management, middleware, and error handling.
* **Database Interaction:** Deep knowledge of using `sqlx` for type-safe, asynchronous database queries, transactions, and connection pooling.
* **Authentication & Authorization:** Skill in implementing JWT-based authentication flows, including token generation, validation, and protecting endpoints.
* **Asynchronous Processing:** Ability to design and implement background job processing within an application, using `tokio` tasks to handle long-running operations without blocking API responses.
* **CLI Application Development:** Experience building sophisticated command-line tools in Rust using a library like `clap` to handle arguments, flags, and subcommands.

**2. Database Engineering & Data Migration**

* **Relational Database Modeling:** The ability to design a clean, normalized PostgreSQL schema that effectively models the domain (users, servers, specs) and improves upon the legacy WHMCS structure.
* **Complex Data Transformation:** The critical skill of analyzing a legacy database schema (WHMCS) and writing transformation logic to map its data—especially complex, unstructured data from "Custom Fields"—into a new, structured schema.
* **ETL (Extract, Transform, Load) Principles:** Understanding the core concepts of data extraction from a source (MySQL), in-memory transformation (in Rust), and loading into a target (PostgreSQL).
* **SQL Proficiency:** Strong SQL skills in both MySQL and PostgreSQL to query the source data effectively and verify the migrated data in the target.
* **Idempotent Scripting:** The ability to write scripts that can be run multiple times with the same outcome, preventing data duplication or corruption.

**3. DevOps & Systems Administration**

* **Containerization:** Expertise in using Docker and Docker Compose to define, build, and run a multiservice local development environment.
* **Legacy System Management:** The practical ability to install, configure, and populate a PHP/MySQL application like WHMCS to serve as a realistic testbed.
* **Database Migration Management:** Proficiency with tools like `sqlx-cli` to manage the evolution of the database schema in a controlled, versioned manner.

**4. Software Architecture & Quality Assurance**

* **Test-Driven Development (TDD):** A strong discipline for writing unit tests for critical business logic (especially the data transformation rules) and integration tests for API endpoints.
* **Mocking & Dependency Injection:** The ability to design code using traits to decouple components, allowing for external services (like a hypervisor API) to be mocked during testing.
* **Structured Logging:** Skill in using the `tracing` library to produce meaningful, structured logs that are essential for debugging a distributed or asynchronous system.
* **Professional Benchmarking:** Expertise in designing, implementing, and analyzing comprehensive performance benchmarks for API endpoints, database operations, and full-system workflows including statistical analysis of performance trends and regression detection.
