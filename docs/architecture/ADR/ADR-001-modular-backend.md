# ADR-001: One Rust workspace, three backend deployables, contexts as modules

Status: Accepted 2026-11-15 (Gate 2 part 1)
Date: 2026-11-15

## Context
TSD v0.1 §3 and §5.1 describe one API service, a device gateway and jobs built from one Rust workspace with a pure calculation library. The engineering skill asks for a microservice-ready design without splitting services that don't need splitting. V1 scale is up to 500 organisations × 2,000 employees (N2), and the database is the main cost and scaling constraint (TSD §19.3).

## Decision
- Deploy three backend units: api, device-gateway and jobs. All come from one workspace.
- Model 14 bounded contexts as separate domain crates with explicit data ownership (`SERVICE_BOUNDARIES.md` §3). Contexts never write each other's data.
- All attendance and leave calculation lives in a pure `core` crate shared by api, jobs, preview and tests.

## Alternatives
- **One service per context now.** Rejected: adds network calls and distributed consistency problems, with no V1 scaling need.
- **A single binary that also handles devices.** Rejected by TSD §3.1: device traffic must not slow the API.

## Consequences
- Moving a context to its own service later means changing in-process calls to remote calls plus outbox events. Data is already separated by owner.
- Requires discipline: a build check that domain crates don't depend on each other's persistence (to define in Phase 5).
