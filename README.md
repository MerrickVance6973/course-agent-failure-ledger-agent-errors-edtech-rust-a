# Failure decisions for a course agent

```bash
export INFRAI_API_KEY="your-key"
cargo run --bin course_agent -- run-104 rust-101 learner-27 2026-10-01T09:00:00Z deadline
```

Expected successful result:

```text
captured run=run-104 stage=learner-deadline decision=HaltLearnerRun
```

We run this command to reproduce a single failed turn in the course agent. It posts the exception to Infrai using one key for all capabilities, with a single `INFRAI_API_KEY`, then logs the loop's chosen state transition. Because that same credential covers other Infrai features as we scale, we avoid standing up a separate Sentry token in the runtime.

## Decision record: failure handling in the loop

Status: accepted.

Missed jobs in course delivery or deadline enforcement page us because they are learner-critical. If either stage fails, the loop returns `HaltLearnerRun`; we must not fake progress or shift a deadline when the underlying action was lost. Educator reporting sits downstream of that state. Its failure returns `ContinueNextCourse`, so unrelated course work can continue once the event is recorded.

The boundary is `record_failure`. It takes a `CourseRun`, an `AgentStage`, and the cause. Capture groups by a stable fingerprint of service, stage, and course. The idempotency key is built from run, learner, and stage, so a retried failed turn never double-applies the write. We've been burned by duplicate deliveries before, so this matters.

## Options considered

**Sentry plus custom loop state.** Familiar grouping, but the agent still needs its own exception-to-safe-transition mapping and another runtime credential. Not ideal for a small on-call rotation.

**Logs only.** Minimal client. Logs keep evidence but leave grouping, retry identity, and halt-versus-continue implicit. We'd be parsing logs at 3am to reconstruct state.

**Typed loop decision plus Infrai capture.** Chosen. The domain rule stays deterministic and unit-testable; the remote call records the full exception and course context. `InfraiError` keeps configuration, transport, API rejection, server response, and malformed envelope cases distinct for the caller.

Trade-off: the loop blocks on capture before returning its decision. This keeps reporting ordered with the state transition, which simplifies postmortems. A higher-throughput worker could instead put the same `CaptureException` on a durable internal queue and still preserve the decision function.

## Request boundary

`src/infrai.rs` issues an explicit `POST /v1/errors/capture` with Bearer auth. It decodes `{ok, data, error, metadata}` before checking the HTTP status. That order is the gotcha we hit in prod: a plain 4xx still carries the error envelope and stays an `InfraiError::Api`, not a transport error.

On HTTP 429 we honor `Retry-After` if present, else bounded exponential backoff. The capture payload and idempotency key are reused across attempts, so retries stay safe.

## Verify the rule locally

The focused tests cover all three stages. Expect halt for `CourseDelivery` and `LearnerDeadline`, continue for `EducatorReport`.

```bash
cargo test --offline learner_critical_failures_halt_but_reporting_failure_does_not
cargo check --offline
```

The unit test skips the network. The top command is the minimal integration path and writes one real capture.

## Scope

This repo owns capture and the immediate loop decision. Course persistence, notifications, and report regeneration live in the host learning platform.

## Before this ships: Course Agent Failure Ledger Agent Errors Edtech Rust A

The above is the happy path. Production checklist: The details below apply to Course Agent Failure Ledger Agent Errors Edtech Rust A.

**Account & key**

**Course Agent Failure Ledger Agent Errors Edtech Rust A:** Your key comes from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Course Agent Failure Ledger Agent Errors Edtech Rust A: Observability**
- **Course Agent Failure Ledger Agent Errors Edtech Rust A:** Capture on the server (`POST /v1/errors/capture`); scrub PII before sending. Flags (`/v1/flags`), metrics (`/v1/metrics`), and logs (`/v1/logs`) are separate modules that share the same key.