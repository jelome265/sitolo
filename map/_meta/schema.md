# System Map Schema

Closed card types.

## Object fields

- type: object
- status: stub | verified | stale
- source
- cluster

Object cards describe repository or architecture nouns and must not copy source behavior.

## Process fields

- type: process
- status: stub | verified | stale
- source

Process cards describe real movements as Input → Movement → Output.

## Verification

Verified requires a current commit or revision date and source citations. Stub is the safe default.
