//! # Project RAG - Paper Library with Semantic Search
//!
//! A Rust library and MCP server for managing a paper library with semantic search
//! using RAG (Retrieval-Augmented Generation).
//!
//! ## Overview
//!
//! Project RAG combines vector embeddings with BM25 keyword search to enable semantic
//! search across papers and documents. It provides paper upload, extraction, and search
//! capabilities through both a Rust library API, an MCP server, and an HTTP web server.
//!
//! ## Architecture
//!
//! - **RagClient**: Core library containing embedding, vector DB, and search functionality
//! - **RagMcpServer**: Thin wrapper around RagClient that exposes functionality via MCP protocol
//! - **Web Server**: HTTP API for paper upload, search, and extraction
//!
//! ## Key Features
//!
//! - **Semantic Search**: Local EmbeddingGemma embeddings with BM25 hybrid search
//! - **Hybrid Search**: Combines vector similarity with BM25 keyword matching (RRF)
//! - **Dual Database Support**: LanceDB (embedded, default) or Qdrant (external server)
//! - **Paper Management**: Upload, extract, and search papers with metadata
//! - **Pattern Extraction**: Extract key patterns and algorithms from papers via Claude CLI
//! - **Dual API**: Use as a Rust library or as an MCP server for AI assistants

/// BM25 keyword search using Tantivy for hybrid search
pub mod bm25_search;

/// Configuration management with environment variable overrides
pub mod config;

/// Embedding generation using local embedding models
pub mod embedding;

/// Error types and utilities
pub mod error;

/// Paper text chunking and PDF extraction
pub mod chunker;

/// Document normalization and structural modeling
pub mod document;

/// Path normalization and utility functions
pub mod paths;

/// Shared tokenizer helpers
pub mod tokenization;

/// Request/response types with validation
pub mod types;

/// Retrieval orchestration built on top of the vector and sparse backends
pub mod retrieval;

/// Vector database abstraction supporting LanceDB and Qdrant
pub mod vector_db;

/// Paper metadata storage (SQLite)
pub mod metadata;

/// HTTP web server for paper upload and search
pub mod web;

/// Pattern extraction via Claude CLI CLI (3-pass pipeline)
pub mod extraction;

// Library client API (core functionality)
pub mod client;
pub use client::RagClient;

// MCP server (wraps the client and exposes via MCP protocol)
pub mod mcp_server;

// Re-export commonly used types for convenience
pub use types::{
    ClearRequest, ClearResponse, LanguageStats, QueryRequest, QueryResponse, SearchResult,
    StatisticsRequest, StatisticsResponse,
};

pub use config::Config;
pub use error::RagError;
