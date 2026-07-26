-- crawlkit-engine PostgreSQL schema

CREATE TABLE IF NOT EXISTS crawls (
    id            TEXT PRIMARY KEY,
    start_time    TIMESTAMPTZ NOT NULL,
    end_time      TIMESTAMPTZ,
    target_url    TEXT NOT NULL,
    pages_crawled INTEGER DEFAULT 0,
    total_issues  INTEGER DEFAULT 0,
    config_json   TEXT
);

CREATE TABLE IF NOT EXISTS pages (
    id            TEXT PRIMARY KEY,
    crawl_id      TEXT NOT NULL REFERENCES crawls(id),
    url           TEXT NOT NULL,
    final_url     TEXT NOT NULL,
    status_code   INTEGER NOT NULL,
    title         TEXT,
    description   TEXT,
    canonical     TEXT,
    word_count    INTEGER,
    load_time_ms  BIGINT,
    body_size     INTEGER,
    fetched_at    TIMESTAMPTZ NOT NULL,
    tenant_id     TEXT,
    etag          TEXT,
    last_modified TEXT,
    cwv_lcp       DOUBLE PRECISION,
    cwv_cls       DOUBLE PRECISION,
    cwv_inp       DOUBLE PRECISION,
    UNIQUE(crawl_id, url)
);

CREATE TABLE IF NOT EXISTS links (
    id            TEXT PRIMARY KEY,
    page_id       TEXT NOT NULL REFERENCES pages(id),
    source_url    TEXT NOT NULL,
    target_url    TEXT NOT NULL,
    anchor_text   TEXT,
    rel           TEXT,
    is_external   BOOLEAN,
    is_nofollow   BOOLEAN
);

CREATE TABLE IF NOT EXISTS findings (
    id            TEXT PRIMARY KEY,
    page_id       TEXT NOT NULL REFERENCES pages(id),
    category      TEXT NOT NULL,
    severity      TEXT NOT NULL,
    code          TEXT NOT NULL,
    title         TEXT NOT NULL,
    description   TEXT NOT NULL,
    element       TEXT,
    recommendation TEXT,
    tenant_id     TEXT
);

CREATE TABLE IF NOT EXISTS images (
    id            TEXT PRIMARY KEY,
    page_id       TEXT NOT NULL REFERENCES pages(id),
    url           TEXT NOT NULL,
    alt           TEXT,
    width         INTEGER,
    height        INTEGER,
    format        TEXT,
    file_size     INTEGER,
    is_lazy_loaded BOOLEAN
);

CREATE TABLE IF NOT EXISTS schemas (
    id            TEXT PRIMARY KEY,
    page_id       TEXT NOT NULL REFERENCES pages(id),
    schema_type   TEXT NOT NULL,
    data_json     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS crux_metrics (
    id            TEXT PRIMARY KEY,
    page_id       TEXT NOT NULL REFERENCES pages(id),
    url           TEXT NOT NULL,
    lcp_p75       DOUBLE PRECISION,
    inp_p75       DOUBLE PRECISION,
    cls_p75       DOUBLE PRECISION,
    fcp_p75       DOUBLE PRECISION,
    ttfb_p75      DOUBLE PRECISION,
    fetched_at    TIMESTAMPTZ NOT NULL,
    UNIQUE(page_id)
);

CREATE INDEX IF NOT EXISTS idx_pages_crawl ON pages(crawl_id);
CREATE INDEX IF NOT EXISTS idx_pages_tenant ON pages(tenant_id);
CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_url);
CREATE INDEX IF NOT EXISTS idx_links_target ON links(target_url);
CREATE INDEX IF NOT EXISTS idx_findings_page ON findings(page_id);
CREATE INDEX IF NOT EXISTS idx_findings_category ON findings(category);
CREATE INDEX IF NOT EXISTS idx_findings_severity ON findings(severity);
CREATE INDEX IF NOT EXISTS idx_findings_tenant ON findings(tenant_id);
