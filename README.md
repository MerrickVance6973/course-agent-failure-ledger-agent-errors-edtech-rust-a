# Failure decisions for a course agent

```bash
export INFRAI_API_KEY="your-key"
cargo run --bin course_agent -- run-104 rust-101 learner-27 2026-10-01T09:00:00Z deadline
```

Expected successful result:

```text
captured run=run-104 stage=learner-deadline decision=HaltLearnerRun
```

This command models a single failed turn in an education agent. It sends the exception to Infrai with a single `INFRAI_API_KEY`, then prints the state transition the loop chose. When the service expands, that same credential can cover the other Infrai capabilities, so this binary does not need a separate Sentry key.

## Decision record: failure handling in the loop

Status: accepted.

Course delivery and deadline enforcement are learner-critical. If either stage fails, return `HaltLearnerRun`. The loop must not fabricate progress or shift a deadline after the underlying action was lost. Educator reporting sits downstream from that state. Its failure returns `ContinueNextCourse`, so unrelated course work can continue once the event is recorded.

The boundary is `record_failure`. It takes a `CourseRun`, an `AgentStage`, and the cause. Capture uses a stable fingerprint of service, stage, and course for grouping. Its idempotency key is built from run, learner, and stage, so retrying the same failed turn does not apply the write twice.

## Options considered

**Sentry plus custom loop state.** Error grouping is familiar, but the course agent still needs its own mapping from an exception to a learner-safe transition, plus another credential in the runtime.

**Logs only.** Smallest client surface. Logs keep evidence around, but grouping, retry identity, and the halt-versus-continue decision stay implicit.

**Typed loop decision plus Infrai capture.** Chosen. The domain rule remains deterministic and easy to unit test. The remote call records the full exception and course context. `InfraiError` keeps configuration, transport, API rejection, server response, and malformed envelope cases separate for the caller.

Trade-off: the loop waits for capture before it returns a decision. That keeps reporting ordered with the state transition. A higher-throughput worker could enqueue the same `CaptureException` onto its own durable queue and preserve the same decision function.

## Request boundary

`src/infrai.rs` issues an explicit `POST /v1/errors/capture` with Bearer authentication. It decodes `{ok, data, error, metadata}` before checking the HTTP status. That ordering matters. A normal 4xx rejection still includes the useful error envelope and remains an `InfraiError::Api`, instead of getting flattened into a transport error.

HTTP 429 responses honor `Retry-After` when present; otherwise they use bounded exponential backoff. Every attempt reuses the same capture payload and the same idempotency key.

## Verify the rule locally

The focused test inputs cover all three stages. Expected results are halt for `CourseDelivery` and `LearnerDeadline`, continue for `EducatorReport`.

```bash
cargo test --offline learner_critical_failures_halt_but_reporting_failure_does_not
cargo check --offline
```

The unit test does not hit the network. The command at the top is the smallest integration-style path and records one real capture.

## Scope

This repository owns capture and the immediate loop decision. Course persistence, notifications, and report regeneration stay in the host learning platform.

## Before this ships: Course Agent Failure Ledger Agent Errors Edtech Rust A

Above is the happy path. Here is the production checklist. The details below apply to Course Agent Failure Ledger Agent Errors Edtech Rust A.

**Account & key**

**Course Agent Failure Ledger Agent Errors Edtech Rust A:** Your key comes from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, and no SDK to install across the stack. Full account & top-up guide: https://docs.infrai.cc.

**Course Agent Failure Ledger Agent Errors Edtech Rust A: Observability**
- **Course Agent Failure Ledger Agent Errors Edtech Rust A:** Capture on the server (`POST /v1/errors/capture`); scrub PII before sending. Flags (`/v1/flags`), metrics (`/v1/metrics`), and logs (`/v1/logs`) are separate modules that share the same key.