//! Arca SQL: Package registry and components.
pub const SCHEMA_PACKAGES: &str = "
CREATE TABLE IF NOT EXISTS packages (
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    hash TEXT NOT NULL REFERENCES objects(hash),
    description TEXT,
    author TEXT,
    license TEXT,
    yanked INTEGER NOT NULL DEFAULT 0,
    published_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (name, version)
);

CREATE TABLE IF NOT EXISTS package_deps (
    package_name TEXT NOT NULL,
    package_version TEXT NOT NULL,
    dep_name TEXT NOT NULL,
    dep_version_req TEXT NOT NULL,
    PRIMARY KEY (package_name, package_version, dep_name),
    FOREIGN KEY (package_name, package_version) REFERENCES packages(name, version)
);

CREATE TABLE IF NOT EXISTS components (
    name TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    schema_hash TEXT REFERENCES objects(hash),
    description TEXT,
    version TEXT NOT NULL DEFAULT '0.1.0',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_packages_hash ON packages(hash);
CREATE INDEX IF NOT EXISTS idx_components_namespace ON components(namespace);
";
