# Telemetry blackout

1. Confirm authoritative database and audit paths remain healthy; telemetry loss must not block business operations.
2. Inspect exporter timeout, queue pressure, dropped-record counters, and bounded local safe logs.
3. Restore collector connectivity and authentication without enabling raw payload logging or unbounded queues.
4. Record the start/end of the gap and assess diagnostic coverage. Do not infer business outcomes from missing telemetry.
5. Confirm recovery returns the buffer to healthy state and add a regression test if the failure mode was new.
