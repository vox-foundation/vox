//! URL and routing path fragments shared by HIR lowering and TS emitters.

/// HTTP path prefix for generated server-function endpoints (`/api/<name>`).
pub const SERVER_FN_API_PREFIX: &str = "/api/";

/// Read-only `@query` endpoints (`/api/query/<name>`), Convex-style data reads.
pub const QUERY_FN_API_PREFIX: &str = "/api/query/";

/// Write `@mutation` endpoints (`/api/mutation/<name>`).
pub const MUTATION_FN_API_PREFIX: &str = "/api/mutation/";
