-- Deprecated compatibility guard. The canonical, versioned migration is
-- migrations/0004_worker_consumer_inbox.sql. This tombstone exists only so
-- historical evidence links resolve; executing it fails closed.
DO $$
BEGIN
    RAISE EXCEPTION 'consumer_inbox.sql was retired; apply schema/migrations/0004_worker_consumer_inbox.sql through the approved migration process';
END
$$;
