PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA auto_vacuum = INCREMENTAL;
PRAGMA temp_store = MEMORY;
PRAGMA page_size = 4096;

BEGIN;

CREATE TABLE IF NOT EXISTS pages (
    id INTEGER PRIMARY KEY,
    url TEXT,
    extract TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS pages_url_extract_idx
    ON pages (url, extract);

CREATE TABLE IF NOT EXISTS links (
    id INTEGER PRIMARY KEY,
    page_id INTEGER REFERENCES pages (id),
    href TEXT,
    text TEXT,
    is_active BOOLEAN DEFAULT TRUE
);

CREATE UNIQUE INDEX IF NOT EXISTS links_page_href_text_idx
    ON links (page_id, href, text);

CREATE TABLE IF NOT EXISTS collections (
    id INTEGER PRIMARY KEY,
    start_time DATETIME,
    end_time DATETIME,
    n_pages INTEGER,
    n_links INTEGER,
    n_new_links INTEGER
);

CREATE TABLE IF NOT EXISTS links_collections (
    collection_id INTEGER REFERENCES collections (id),
    link_id INTEGER REFERENCES links (id),
    timestamp DATETIME
);

CREATE INDEX IF NOT EXISTS links_collections_collection_idx
    ON links_collections (collection_id);
CREATE INDEX IF NOT EXISTS links_collections_link_idx
    ON links_collections (link_id);

COMMIT;
