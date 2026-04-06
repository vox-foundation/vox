use serde::{Deserialize, Serialize};

// ── Public Types ────────────────────────────────────────

/// Complete schema digest — the single source of truth for LLM context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDigest {
    /// All declared tables with fields and metadata.
    pub tables: Vec<TableInfo>,
    /// All declared schemaless document collections.
    pub collections: Vec<CollectionInfo>,
    /// Auto-detected relationships from `Id<X>` references.
    pub relationships: Vec<Relationship>,
    /// All declared indexes (standard, vector, search).
    pub indexes: Vec<IndexInfo>,
    /// All declared queries.
    pub queries: Vec<FunctionInfo>,
    /// All declared mutations.
    pub mutations: Vec<FunctionInfo>,

    /// Human-readable summary for LLM prompts.
    pub summary: String,
    /// The exact VCS context snapshot ID this schema belongs to.
    pub vcs_snapshot_id: Option<String>,
}

/// Information about a single database table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// Vox `@table` name.
    pub name: String,
    /// Declared columns / fields.
    pub fields: Vec<FieldInfo>,
    /// User-provided description (from `@describe` decorator, if present).
    pub description: Option<String>,
    /// Auto-generated example insert statement.
    pub example_insert: String,
    /// Auto-generated example query statement.
    pub example_query: String,
    /// Whether public (`is_pub`).
    pub is_public: bool,
    /// Auth provider if configured.
    pub auth_provider: Option<String>,
    /// Sample data (first 3 rows) if populated by the context engine.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sample_data: Vec<serde_json::Value>,
}

/// Information about a single document collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    /// Collection / table stem as declared in Vox.
    pub name: String,
    /// Inferred or declared field shapes for LLM context.
    pub fields: Vec<FieldInfo>,
    /// User-provided description (from `@describe` decorator, if present).
    pub description: Option<String>,
    /// Whether the collection is public API surface.
    pub is_public: bool,
    /// Sample data (first 3 rows) if populated by the context engine.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sample_data: Vec<serde_json::Value>,
}

/// Information about a single table field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    /// Column / field name in Vox source.
    pub name: String,
    /// Human-readable type string (e.g. "str", "int", "Option[str]").
    pub type_str: String,
    /// Whether this field is optional.
    pub is_optional: bool,
    /// Whether this field references another table (via `Id<X>`).
    pub references_table: Option<String>,
}

/// A detected relationship between tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Table that holds the foreign key.
    pub from_table: String,
    /// Field that contains the reference.
    pub from_field: String,
    /// Table being referenced.
    pub to_table: String,
    /// Kind of relationship.
    pub kind: RelationshipKind,
}

/// Relationship kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipKind {
    /// `Id[X]` → references one record in table X.
    OneToOne,
    /// `List[Id[X]]` → references many records in table X.
    OneToMany,
    /// Inferred from field name matching a table name.
    Inferred,
}

/// Information about an index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    /// Table this index is attached to.
    pub table_name: String,
    /// Declared index name in Vox.
    pub index_name: String,
    /// Standard, vector, or search index.
    pub kind: IndexKind,
    /// Indexed columns or vector/search field names.
    pub columns: Vec<String>,
    /// For vector indexes: dimensionality.
    pub dimensions: Option<u32>,
    /// For vector/search indexes: filter fields.
    pub filter_fields: Vec<String>,
}

/// The kind of index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexKind {
    /// B-tree style on listed columns.
    Standard,
    /// Vector / embedding index with [`IndexInfo::dimensions`].
    Vector,
    /// Full-text or search-field index.
    Search,
}

/// Information about a query, mutation, or action function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    /// Function name in Vox source.
    pub name: String,
    /// Formal parameters.
    pub params: Vec<ParamInfo>,
    /// Pretty-printed return type, if any.
    pub return_type: Option<String>,
    /// Tables this function likely reads from (for queries) or writes to (for mutations).
    pub affected_tables: Vec<String>,
}

/// A function parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamInfo {
    /// Parameter name in source.
    pub name: String,
    /// Pretty-printed type for LLM context.
    pub type_str: String,
}
