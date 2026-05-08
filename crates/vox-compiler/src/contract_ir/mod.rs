//! Contract IR — the wire-format projection of HIR.
//!
//! Sits between [`crate::hir`] and the wire-format emitters
//! ([`crate::codegen_ts::zod_emit`], the future OpenAPI emitter, JSON Schema, the
//! TS client SDK). Each consumer reads `ContractIr` rather than walking HIR
//! independently — so the wire-format-v1 rules
//! ([`docs/src/architecture/wire-format-v1-ssot.md`](../../../../docs/src/architecture/wire-format-v1-ssot.md))
//! are enforced in one place.
//!
//! See [`docs/src/architecture/frontend-convergence-findings-2026.md`](../../../../docs/src/architecture/frontend-convergence-findings-2026.md)
//! §Convergence design for the role this layer plays.
//!
//! # What is in scope
//!
//! - `HirTypeDef` → [`ContractType`] (struct, sum, alias)
//! - `HirEndpointFn` → [`ContractEndpoint`] (HTTP method, path, request/response shape)
//! - `HirType` → [`WireType`] (Decimal/BigInt → string, Option → optional, etc.)
//!
//! # What is *not* in scope
//!
//! - GUI emit (Web IR owns that)
//! - Component lowering
//! - Server-side function bodies — Contract IR carries shapes only
//!
//! # Stability
//!
//! Contract IR is the seam Phase 2 of the
//! [external frontend interop plan](../../../../docs/src/architecture/external-frontend-interop-plan-2026.md)
//! plugs OpenAPI emit into. Adding fields here is additive; renaming is a
//! breaking change for every wire-format emitter.

use crate::hir::{HirEndpointFn, HirEndpointKind, HirModule, HirType};

pub mod project;

#[cfg(test)]
mod tests;

/// The wire-format projection of a Vox HIR module.
#[derive(Debug, Clone, Default)]
pub struct ContractIr {
    /// Type contracts in declaration order.
    pub types: Vec<ContractType>,
    /// Endpoint contracts in declaration order.
    pub endpoints: Vec<ContractEndpoint>,
}

/// A single named type at the wire boundary.
#[derive(Debug, Clone)]
pub struct ContractType {
    pub name: String,
    pub kind: ContractTypeKind,
}

/// Shape of a [`ContractType`].
#[derive(Debug, Clone)]
pub enum ContractTypeKind {
    /// Product type — flat object with named fields.
    Struct { fields: Vec<ContractField> },
    /// Sum type — `_tag`-discriminated union (per wire-format-v1).
    Sum { variants: Vec<ContractVariant> },
}

/// A field on a struct or variant.
#[derive(Debug, Clone)]
pub struct ContractField {
    pub name: String,
    pub ty: WireType,
    /// `Option<T>` projects as `optional = true` and is encoded as
    /// presence-or-absence of the JSON key (per wire-format-v1).
    pub optional: bool,
}

/// One variant of a sum type.
#[derive(Debug, Clone)]
pub struct ContractVariant {
    /// Value of the `_tag` discriminant for this variant.
    pub tag: String,
    pub fields: Vec<ContractField>,
}

/// Wire-level type alphabet.
///
/// Reflects the encoding rules of wire-format-v1:
/// - `Decimal` → string (precision-preserving)
/// - `BigInt` → string (JSON Number can't hold > 2^53 safely)
/// - `Date` / `DateTime` → RFC 3339 UTC string
/// - `Option<T>` → presence-or-absence of key (carried on [`ContractField::optional`])
#[derive(Debug, Clone)]
pub enum WireType {
    /// JSON Number — int / float scalars.
    Number,
    /// JSON string.
    String,
    /// JSON boolean.
    Bool,
    /// JSON-encoded as a string per wire-format-v1.
    DecimalString,
    /// JSON-encoded as a string per wire-format-v1.
    BigIntString,
    /// JSON-encoded as an RFC 3339 UTC string per wire-format-v1.
    DateTimeString,
    /// Homogeneous array.
    Array(Box<WireType>),
    /// Reference to another `ContractType` by name.
    Ref(String),
    /// Heterogeneous tuple.
    Tuple(Vec<WireType>),
    /// Void / unit (no value at the wire).
    Unit,
    /// Fallback for unrepresentable Vox types — emitters should treat as opaque.
    Unknown,
}

/// A request-response endpoint at the wire boundary.
#[derive(Debug, Clone)]
pub struct ContractEndpoint {
    pub kind: ContractEndpointKind,
    pub name: String,
    pub method: HttpMethod,
    pub path: String,
    /// Function parameters projected as request fields. For GET, callers send
    /// these as query params; for POST, as a JSON body.
    pub params: Vec<ContractField>,
    pub response: WireType,
    /// Whether the source declared `@pure` — a hint for caching, not a
    /// load-bearing wire-format property.
    pub is_pure: bool,
}

/// Mirrors [`HirEndpointKind`] without exposing it as a public name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractEndpointKind {
    Query,
    Mutation,
    Server,
}

/// HTTP method bound to an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
        }
    }
}

/// Project a HIR module into Contract IR.
///
/// This is the single wire-format-aware traversal of HIR. All wire-format
/// emitters should consume the result rather than walking HIR themselves.
pub fn project(hir: &HirModule) -> ContractIr {
    ContractIr {
        types: hir.types.iter().map(project::type_def).collect(),
        endpoints: hir.endpoint_fns.iter().map(project::endpoint).collect(),
    }
}

/// Map a [`HirType`] to its wire representation.
///
/// Public so emitters can lift function-parameter and return types into the
/// same wire alphabet without re-implementing the rules.
pub fn project_type(ty: &HirType) -> WireType {
    project::ty(ty)
}

impl From<HirEndpointKind> for ContractEndpointKind {
    fn from(k: HirEndpointKind) -> Self {
        match k {
            HirEndpointKind::Query => ContractEndpointKind::Query,
            HirEndpointKind::Mutation => ContractEndpointKind::Mutation,
            HirEndpointKind::Server => ContractEndpointKind::Server,
        }
    }
}

impl ContractEndpointKind {
    /// Default HTTP method for a given endpoint kind.
    ///
    /// `Query` is GET (cacheable, idempotent, no body). `Mutation` and
    /// `Server` are POST. Phase 3 (`@endpoint(method: …)`) will let users
    /// override this.
    pub fn default_method(self) -> HttpMethod {
        match self {
            ContractEndpointKind::Query => HttpMethod::Get,
            ContractEndpointKind::Mutation | ContractEndpointKind::Server => HttpMethod::Post,
        }
    }
}

/// Borrow a Hir endpoint as a [`ContractEndpoint`] without owning it.
///
/// Convenience for emitters that want to feed a single endpoint through the
/// wire-format projection without going via a full module.
pub fn project_endpoint(e: &HirEndpointFn) -> ContractEndpoint {
    project::endpoint(e)
}
