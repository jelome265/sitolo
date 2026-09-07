# Release Governance Evidence (Audit F-022)

Snapshot of repository-administration controls that live outside source
files. Exported read-only via `gh api`; contains no secrets.
Exported: 2026-09-07. Refresh before any production certification.

## Branch protection (`main`)

- Status: **ABSENT** — `GET /repos/jelome265/sitolo/branches/main/protection`
  returns HTTP 404.
- Consequence: no required status checks, no required reviews, no
  force-push/deletion policy is currently enforced by GitHub. The workflow
  files alone cannot prove these settings.
- Required action (repository admin): enable branch protection on `main`
  with the canonical verification jobs as required checks, at least one
  required review, force-push disabled, deletion disabled, and admin
  enforcement on.

## Environments

- Status: **NONE** — `total_count: 0`.
- Consequence: `release.yml` references `environment: production`, but no
  `production` environment exists, so the intended approval gate currently
  enforces nothing.
- Required action (repository admin): create a `production` environment
  with required approvers and deployment-branch restriction to protected
  tags before any production promotion.

## Repository settings (read-only snapshot)

- Default branch: `main`. Visibility: public. Archived: false.
- Merge methods: merge, squash, and rebase all allowed.
- Delete branch on merge: true.

## Actions permissions

- Repository Actions permissions endpoints returned HTTP 403 for the
  export token (insufficient scope), so the workflow default-permissions
  policy could not be snapshotted here.
- Required action (repository admin): confirm restrictive workflow default
  permissions in repository settings; record the values in the next
  refresh of this file.

## Workflow permission policy (source-level, verified)

- `policy.yml`, `rust.yml`, `integration.yml`: `contents: read`.
- `security.yml`: `contents: read` (job-scoped `security-events: write`
  only where CodeQL/gitleaks require it).
- `artifact.yml`: workflow-level `contents: read`; release job holds
  `contents: write` + `id-token: write` + `attestations: write`.
- `release.yml`: `contents: read` + `attestations: read`.
