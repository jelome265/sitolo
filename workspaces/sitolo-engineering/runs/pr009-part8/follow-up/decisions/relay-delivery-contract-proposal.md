# Proposed Relay and Consumer Contract

**Status:** **PROPOSED — NOT APPROVED**  
**Scope:** Phase 4 Part 8 IAM integration events. The binding contract remains [phase4_part8_audit_outbox_implementation_contract.md](../../../../../../docs/phase4_part8_audit_outbox_implementation_contract.md).

## Decision requested

Use one Amazon SNS FIFO topic for the IAM event stream and one Amazon SQS FIFO queue for every consuming service. Provision a **main consumer** queue and an independent queue for each **sub-consumer**. These are fan-out subscribers, not a new authority hierarchy. This proposal selects AWS because managed FIFO fan-out fits the current bounded outbox worker without introducing Kafka/RabbitMQ operations. It also introduces real AWS cost and vendor lock-in; the owner must explicitly accept that trade-off.

Proposed region: `af-south-1` (AWS Africa/Cape Town), subject to service-level availability checks, measured latency, cost and data-handling review. Regional availability is not a legal-residency finding.

## Topology

```text
PostgreSQL transaction: business + mandatory audit + outbox
                         |
                         v
                Sitolo relay worker
                         |
                         v
           sitolo-iam-events-<env>.fifo
                    SNS FIFO topic
                  /               \
                 v                 v
      main-consumer.fifo    sub-consumer-<name>.fifo
            SQS FIFO               SQS FIFO
                 |                 |
                 v                 v
      durable inbox + effect   durable inbox + effect
                 |                 |
          delete after commit  delete after commit
```

Each consumer has its own FIFO queue and FIFO dead-letter queue. Infrastructure creates topic, queues, subscriptions, encryption and resource policies. The producer can publish only to the topic; it cannot consume/delete from subscriber queues. Do not enable a real adapter or create cloud resources until the owner approves the contract.

## Published envelope (v1)

Publish immutable event data only. Do not serialize the complete internal `OutboxEvent`: delivery status, claim token/generation, retries, lease timestamps and last-error metadata are not event payload fields.

| Field | Requirement |
|---|---|
| `event_id` | Globally unique and stable; also SNS `MessageDeduplicationId` |
| `event_name` | Registered lowercase dotted name |
| `event_version` | Positive schema version, initially `1` |
| `occurred_at` | Persisted event timestamp in UTC |
| `organization_id` | Required server-validated tenant scope |
| `branch_id` | Nullable branch scope |
| `aggregate_type`, `aggregate_id` | Canonical aggregate identity |
| `aggregate_sequence` | Database-assigned sequence, required for ordered aggregate event families |
| `payload` | Bounded, validated, secret-free event JSON |

Retain the existing **65,536-byte** application payload ceiling. Incompatible payload changes require a new schema version and a tested compatibility path. Unknown names/versions, invalid schemas and malformed payloads fail closed.

Payloads must not include raw invitation tokens, token hashes, access/refresh credentials, passwords, private keys, DSNs, unrestricted debug output or unnecessary personal data. Event delivery never grants tenant authority.

## Producer and consumer acknowledgements

**Producer:** mark the outbox row `PUBLISHED` only after SNS acknowledges that it accepted the message and returns its message identifier. In this contract, `PUBLISHED` means accepted by SNS, **not** processed by each consumer.

**Consumer:** insert a durable inbox record keyed uniquely by `(consumer_id, event_id)` and apply its business effect in the same local transaction. Delete/ack the SQS message only after that transaction commits. A duplicate event is recognized through the durable inbox and its effect is not repeated.

End-to-end behavior is **at least once**, not exactly once. SNS/SQS FIFO service-side deduplication is bounded (five minutes); the consumer inbox remains necessary for later replays and crash windows. See [SQS FIFO identifiers](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-fifo-queue-message-identifiers.html) and [outage recovery guidance](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/designing-for-outage-recovery-scenarios.html).

## Ordering

Guarantee ordering **per aggregate**, not global order across all organizations.

- Derive `MessageGroupId` from a stable hash of `organization_id + NUL + aggregate_type + NUL + aggregate_id`.
- Use `MessageDeduplicationId = event_id` on every retry.
- The database claim query and worker must ensure a higher aggregate sequence cannot publish while an earlier sequence is pending, claimed, retrying or quarantined. Allow at most one publish in flight per aggregate. FIFO preserves send order; it cannot repair out-of-order sends by concurrent publishers.
- A quarantined sequence blocks later events for that aggregate until an authorized operator repairs/replays it or records an explicitly audited skip decision.
- Different aggregates may publish concurrently.

This behavior requires PostgreSQL-backed concurrency tests; a fake-publisher unit test is insufficient.

## Authentication, configuration and least privilege

Use the AWS SDK workload-credential chain from the ECS task role, never long-lived production `AWS_ACCESS_KEY_ID`/`AWS_SECRET_ACCESS_KEY` values.

- Relay role: `sns:Publish` only on the approved topic ARN. No topic administration, secret administration or SQS consumer permissions.
- Each consumer role: receive/change-visibility/delete only on its own SQS queue and its own DLQ operations. No broad queue/topic administration.
- Queue resource policies allow delivery from the specific topic. Encrypt SNS/SQS at rest; if using a customer-managed KMS key, scope permissions to the actual resources/principals.
- Require TLS to AWS APIs. No arbitrary publish URL or arbitrary outbound fetch.
- Proposed typed config names (not currently registered in `sitolo-config`): `SITOLO__EVENTS__RELAY__AWS_REGION`, `SITOLO__EVENTS__RELAY__SNS_TOPIC_ARN`, `SITOLO__EVENTS__RELAY__BATCH_SIZE`, `SITOLO__EVENTS__RELAY__MAX_ATTEMPTS`, `SITOLO__EVENTS__RELAY__BACKOFF_MAX_MS`. Add them intentionally to the strict schema before use.
- Region/topic ARNs are non-secret configuration. No AWS role credential is copied into the Sitolo secret namespace.

## Retry, quarantine and operational failure

Retain the existing ceilings unless approved tests justify a change: at most **10 attempts**, exponential backoff capped at **1 hour**, with **full jitter** (add jitter if the final code does not yet do so).

| Outcome | Handling |
|---|---|
| Network timeout/reset, throttling, temporary service 5xx | Retry same event ID with bounded backoff |
| Malformed event, unknown event name/version, per-event permanent rejection | Quarantine that event and alert |
| `AccessDenied`, missing topic/region or a worker-wide configuration/permission defect | Mark worker unhealthy and alert; do not quarantine all pending events as individually poisonous |
| Unknown response outcome | Treat as ambiguous; retry the identical event ID within the bound, then require explicit operator handling |

Each SQS FIFO consumer queue gets its own FIFO DLQ. Proposed `maxReceiveCount=5`, with alerts on DLQ depth/age. Consumer DLQ and outbox `QUARANTINED` are separate states. Re-drive preserves event ID and is durably audited. Manual replay and sequence-skip require restricted operator permissions, a reason and an audit record; never silently delete a failed event.

## Ownership

- Product/platform architecture owner: approves main/sub subscriber set and delivery semantics.
- Platform operations: AWS region/account, resource policies, KMS, alarms, retention, replay authority and incident runbook.
- Each consumer owner: schema compatibility, inbox persistence, business-effect transaction and DLQ.
- Security owner: IAM policies, tenant scope, payload fields, secret access and privilege boundaries.

Use aggregate metrics without organization/user IDs or payloads as labels: pending count/oldest age, publish latency/success, retries, quarantine, failure class, consumer queue age/depth, DLQ depth and replay count.

## Release acceptance tests

1. Real publisher confirms SNS acceptance and correct error classification.
2. Integration proves topic fan-out to main and at least one sub-consumer queue.
3. Consumer test proves inbox + side effect atomicity and deduplication after crash between commit and delete.
4. PostgreSQL concurrency tests prove per-aggregate order, claim fencing, retry, recovery and quarantine-gap blocking.
5. Cover timeout after accepted publish, same-ID resend, throttling, IAM denial, schema mismatch, DLQ routing and audited replay.
6. Verify the actual IAM/resource policies and source/infrastructure revision.
7. Pin the evidence to the exact source head.

## References

- [SNS FIFO delivery to SQS subscriptions](https://docs.aws.amazon.com/sns/latest/dg/fifo-message-delivery.html)
- [SNS FIFO ordering](https://docs.aws.amazon.com/sns/latest/dg/fifo-topic-message-ordering.html)
- [SQS FIFO message identifiers](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-fifo-queue-message-identifiers.html)
- [SQS outage recovery](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/designing-for-outage-recovery-scenarios.html)
