mod model;
mod normalize;

pub use model::{NodeKind, NormalizedDocument, SourceKind, StructuralNode};
pub use normalize::{normalize_chunk_input, normalize_pdf_markdown};
