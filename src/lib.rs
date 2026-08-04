//! # seekstorm_client_py
//!
//! Python bindings for the SeekStorm REST client library.
//!
//! SeekStorm is an open-source, sub-millisecond vector and lexical search library & multi-tenancy server written in Rust.
//! This crate provides Python bindings to interact with a SeekStorm server via its REST API.
//!
//! ## Quick Start
//!
//! ```python
//! from seekstorm_client_py import SeekStormClient, SearchRequestObject
//!
//! client = SeekStormClient()
//!
//! # Check if server is live
//! result = client.live("http://127.0.0.1:80")
//!
//! # Create a search request
//! request = SearchRequestObject("search query")
//! request.offset = 0
//! request.length = 10
//!
//! # Execute a search
//! results = client.query_index("http://127.0.0.1:80", "your-api-key", 1, request)
//! print(f"Found {results.count_total} results")
//! ```
//!
//! ## Core Classes
//!
//! - `SeekStormClient`: Main client for REST API access
//! - `SearchRequestObject`: Search query parameters
//! - `SearchResultObject`: Search results from query_index
//! - `GetIteratorRequest`: Document iterator parameters
//! - `IteratorResult`: Document iterator results with items
//! - `IteratorResultItem`: Individual document from iterator
//! - `GetDocumentRequest`: Parameters for retrieving a single document
//! - `ApikeyQuotaObject`: API key quota and rate limit settings
//! - `CreateIndexRequest`: Index creation parameters with schema and configuration
//!
//! ## API Methods
//!
//! - **API Key Management**: `create_apikey`, `delete_apikey`, `get_apikey_info`
//! - **Index Management**: `create_index`, `delete_index`, `clear_index`, `commit_index`, `get_index_info`
//! - **Document Operations**: `index_document`, `index_documents`, `index_pdf`, `get_pdf`, `get_document`, `update_document`, `update_documents`, `delete_document_by_docid`, `delete_documents_by_docid`
//! - **Search & Iteration**: `query_index`, `delete_documents_by_query`, `document_iterator`
//! - **Server**: `live`

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use seekstorm_client_rs::{
    ApikeyQuotaObject, CreateIndexRequest, Document, DocumentCompression, GetDocumentRequest,
    GetIteratorRequest, IteratorResult, IteratorResultItem, LexicalSimilarity, RestClient,
    SearchRequestObject, SearchResultObject, StemmerType, TokenizerType,
};
use std::path::Path;

fn to_json_string<T: serde::Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string(value)
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize JSON: {}", e)))
}

fn from_json_str<T: serde::de::DeserializeOwned>(json: &str, context: &str) -> PyResult<T> {
    serde_json::from_str(json)
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to parse {} JSON: {}", context, e)))
}

fn parse_count_response(body: &str) -> Result<usize, String> {
    let trimmed = body.trim();
    if let Ok(value) = trimmed.parse::<usize>() {
        return Ok(value);
    }

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(value) = json.as_u64() {
            return Ok(value as usize);
        }

        if let Some(object) = json.as_object() {
            for key in ["Ok", "ok", "count", "count_total"] {
                if let Some(value) = object.get(key).and_then(|entry| entry.as_u64()) {
                    return Ok(value as usize);
                }
            }
        }
    }

    Err("Failed to parse response as usize".to_string())
}

fn parse_u64_response(body: &str) -> Result<u64, String> {
    let trimmed = body.trim();
    if let Ok(value) = trimmed.parse::<u64>() {
        return Ok(value);
    }

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(value) = json.as_u64() {
            return Ok(value);
        }

        if let Some(object) = json.as_object() {
            for key in ["Ok", "ok", "count", "count_total"] {
                if let Some(value) = object.get(key).and_then(|entry| entry.as_u64()) {
                    return Ok(value);
                }
            }
        }
    }

    Err("Failed to parse response as u64".to_string())
}

// Thread-local tokio runtime shared across all method calls to avoid per-call allocation overhead.
// Leak the runtime for the process lifetime so its worker threads are never torn down during exit.
thread_local! {
    static RUNTIME: &'static tokio::runtime::Runtime =
        Box::leak(Box::new(tokio::runtime::Runtime::new().expect("Failed to create tokio runtime")));
}

/// Python wrapper for SearchRequestObject
///
/// Represents a search query with configurable parameters for executing searches against a SeekStorm index.
///
/// Properties:
/// * `query_string`: The search query text (required, set via constructor)
/// * `query_vector`: Optional vector query (JSON-based setter/getter)
/// * `offset`: Result offset for pagination (default: 0)
/// * `length`: Number of results to return (default: 10)
/// * `enable_empty_query`: Allow searches with empty query string (default: false)
/// * `result_type`: Result mode such as Topk/Count/TopkCount
/// * `realtime`: Enable real-time search mode (default: false)
/// * `highlights`, `field_filter`, `fields`, `distance_fields`, `query_facets`,
///   `facet_filter`, `result_sort`, `query_type_default`, `query_rewriting`, `search_mode`:
///   Advanced fields exposed via typed or JSON-based setters/getters.
///
/// # Examples
///
/// ```python
/// from seekstorm_client_py import SearchRequestObject
///
/// # Create a search request
/// request = SearchRequestObject("search term")
/// request.offset = 10
/// request.length = 20
/// request.enable_empty_query = False
/// request.realtime = True
/// ```
///
/// Notes:
/// * Complex nested fields are exposed with JSON helper methods.
/// * JSON helpers accept/return the same payload shapes used by the REST API.
#[pyclass(name = "SearchRequestObject")]
pub struct PySearchRequestObject {
    pub inner: SearchRequestObject,
}

#[pymethods]
impl PySearchRequestObject {
    /// Create a new SearchRequestObject with a query string
    ///
    /// Arguments:
    /// * `query_string`: The search query text
    ///
    /// Returns:
    /// * `SearchRequestObject`: New request with defaults (`offset=0`, `length=10`, `realtime=false`)
    ///
    /// Example:
    /// ```python
    /// req = SearchRequestObject("rust bindings")
    /// req.offset = 0
    /// req.length = 25
    /// ```
    ///
    #[new]
    #[pyo3(signature = (query_string), text_signature = "(query_string)")]
    fn new(query_string: String) -> Self {
        PySearchRequestObject {
            inner: SearchRequestObject {
                query_string,
                query_vector: None,
                enable_empty_query: false,
                offset: 0,
                length: 10,
                result_type: Default::default(),
                realtime: false,
                highlights: Vec::new(),
                field_filter: Vec::new(),
                fields: Vec::new(),
                distance_fields: Vec::new(),
                query_facets: Vec::new(),
                facet_filter: Vec::new(),
                result_sort: Vec::new(),
                query_type_default: Default::default(),
                query_rewriting: Default::default(),
                search_mode: Default::default(),
            },
        }
    }

    /// Get the search query text.
    ///
    /// Returns:
    /// * `str`: Current query string.
    #[getter]
    fn query_string(&self) -> String {
        self.inner.query_string.clone()
    }

    /// Set the search query text.
    ///
    /// Arguments:
    /// * `value`: Query string to execute.
    #[setter]
    fn set_query_string(&mut self, value: String) {
        self.inner.query_string = value;
    }

    /// Get query vector as JSON string.
    ///
    /// Returns:
    /// * `str`: JSON for optional vector, e.g. `null` or `[0.12, 0.34]`.
    #[getter]
    fn query_vector(&self) -> PyResult<String> {
        to_json_string(&self.inner.query_vector)
    }

    /// Set query vector from JSON string.
    ///
    /// Arguments:
    /// * `value`: JSON for optional vector, e.g. `null` or `[0.12, 0.34]`.
    ///
    /// Raises:
    /// * `RuntimeError`: If JSON parsing fails.
    #[setter]
    fn set_query_vector(&mut self, value: String) -> PyResult<()> {
        self.inner.query_vector = from_json_str(&value, "query_vector")?;
        Ok(())
    }

    /// Get the zero-based result offset for pagination.
    #[getter]
    fn offset(&self) -> usize {
        self.inner.offset
    }

    /// Set the zero-based result offset for pagination.
    ///
    /// Arguments:
    /// * `value`: Number of initial results to skip.
    #[setter]
    fn set_offset(&mut self, value: usize) {
        self.inner.offset = value;
    }

    /// Get the number of results to return.
    #[getter]
    fn length(&self) -> usize {
        self.inner.length
    }

    /// Set the number of results to return.
    ///
    /// Arguments:
    /// * `value`: Page size for this query.
    ///
    /// Example:
    /// ```python
    /// req.length = 50
    /// ```
    #[setter]
    fn set_length(&mut self, value: usize) {
        self.inner.length = value;
    }

    /// Get whether empty query strings are allowed.
    #[getter]
    fn enable_empty_query(&self) -> bool {
        self.inner.enable_empty_query
    }

    /// Set whether empty query strings are allowed.
    ///
    /// Arguments:
    /// * `value`: `true` to allow empty queries, `false` to require non-empty queries.
    #[setter]
    fn set_enable_empty_query(&mut self, value: bool) {
        self.inner.enable_empty_query = value;
    }

    /// Get whether real-time search mode is enabled.
    #[getter]
    fn realtime(&self) -> bool {
        self.inner.realtime
    }

    /// Set whether real-time search mode is enabled.
    ///
    /// Arguments:
    /// * `value`: `true` to include uncommitted changes, `false` for committed-state queries.
    #[setter]
    fn set_realtime(&mut self, value: bool) {
        self.inner.realtime = value;
    }

    /// Get result type as string.
    #[getter]
    fn result_type(&self) -> String {
        format!("{:?}", self.inner.result_type)
    }

    /// Set result type.
    ///
    /// Allowed values:
    /// * `"Count"`
    /// * `"Topk"`
    /// * `"TopkCount"`
    ///
    /// Raises:
    /// * `RuntimeError`: If `value` is invalid.
    #[setter]
    fn set_result_type(&mut self, value: String) -> PyResult<()> {
        self.inner.result_type = match value.as_str() {
            "Count" => from_json_str("\"Count\"", "result_type")?,
            "Topk" => from_json_str("\"Topk\"", "result_type")?,
            "TopkCount" => from_json_str("\"TopkCount\"", "result_type")?,
            _ => return Err(PyRuntimeError::new_err("Invalid result_type")),
        };
        Ok(())
    }

    /// Get field_filter values.
    #[getter]
    fn field_filter(&self) -> Vec<String> {
        self.inner.field_filter.clone()
    }

    /// Set field_filter values.
    #[setter]
    fn set_field_filter(&mut self, value: Vec<String>) {
        self.inner.field_filter = value;
    }

    /// Get return fields.
    #[getter]
    fn fields(&self) -> Vec<String> {
        self.inner.fields.clone()
    }

    /// Set return fields.
    #[setter]
    fn set_fields(&mut self, value: Vec<String>) {
        self.inner.fields = value;
    }

    /// Get highlights as JSON string.
    #[getter]
    fn highlights(&self) -> PyResult<String> {
        to_json_string(&self.inner.highlights)
    }

    /// Set highlights from JSON string.
    #[setter]
    fn set_highlights(&mut self, value: String) -> PyResult<()> {
        self.inner.highlights = from_json_str(&value, "highlights")?;
        Ok(())
    }

    /// Get distance_fields as JSON string.
    #[getter]
    fn distance_fields(&self) -> PyResult<String> {
        to_json_string(&self.inner.distance_fields)
    }

    /// Set distance_fields from JSON string.
    #[setter]
    fn set_distance_fields(&mut self, value: String) -> PyResult<()> {
        self.inner.distance_fields = from_json_str(&value, "distance_fields")?;
        Ok(())
    }

    /// Get query_facets as JSON string.
    #[getter]
    fn query_facets(&self) -> PyResult<String> {
        to_json_string(&self.inner.query_facets)
    }

    /// Set query_facets from JSON string.
    #[setter]
    fn set_query_facets(&mut self, value: String) -> PyResult<()> {
        self.inner.query_facets = from_json_str(&value, "query_facets")?;
        Ok(())
    }

    /// Get facet_filter as JSON string.
    #[getter]
    fn facet_filter(&self) -> PyResult<String> {
        to_json_string(&self.inner.facet_filter)
    }

    /// Set facet_filter from JSON string.
    #[setter]
    fn set_facet_filter(&mut self, value: String) -> PyResult<()> {
        self.inner.facet_filter = from_json_str(&value, "facet_filter")?;
        Ok(())
    }

    /// Get result_sort as JSON string.
    #[getter]
    fn result_sort(&self) -> PyResult<String> {
        to_json_string(&self.inner.result_sort)
    }

    /// Set result_sort from JSON string.
    #[setter]
    fn set_result_sort(&mut self, value: String) -> PyResult<()> {
        self.inner.result_sort = from_json_str(&value, "result_sort")?;
        Ok(())
    }

    /// Get default query type as string.
    #[getter]
    fn query_type_default(&self) -> String {
        format!("{:?}", self.inner.query_type_default)
    }

    /// Set default query type.
    ///
    /// Allowed values:
    /// * `"Union"`
    /// * `"Intersection"`
    /// * `"Phrase"`
    /// * `"Not"`
    ///
    /// Raises:
    /// * `RuntimeError`: If `value` is invalid.
    #[setter]
    fn set_query_type_default(&mut self, value: String) -> PyResult<()> {
        self.inner.query_type_default = match value.as_str() {
            "Union" => from_json_str("\"Union\"", "query_type_default")?,
            "Intersection" => from_json_str("\"Intersection\"", "query_type_default")?,
            "Phrase" => from_json_str("\"Phrase\"", "query_type_default")?,
            "Not" => from_json_str("\"Not\"", "query_type_default")?,
            _ => return Err(PyRuntimeError::new_err("Invalid query_type_default")),
        };
        Ok(())
    }

    /// Get query_rewriting as JSON string.
    #[getter]
    fn query_rewriting(&self) -> PyResult<String> {
        to_json_string(&self.inner.query_rewriting)
    }

    /// Set query_rewriting from JSON string.
    #[setter]
    fn set_query_rewriting(&mut self, value: String) -> PyResult<()> {
        self.inner.query_rewriting = from_json_str(&value, "query_rewriting")?;
        Ok(())
    }

    /// Get search_mode as JSON string.
    #[getter]
    fn search_mode(&self) -> PyResult<String> {
        to_json_string(&self.inner.search_mode)
    }

    /// Set search_mode from JSON string.
    #[setter]
    fn set_search_mode(&mut self, value: String) -> PyResult<()> {
        self.inner.search_mode = from_json_str(&value, "search_mode")?;
        Ok(())
    }
}

/// Python wrapper for SearchResultObject
///
/// Contains the results of a search query execution.
/// All properties are read-only and populated by the server.
///
/// Properties:
/// * `time`: Execution time in microseconds
/// * `query`: The processed query string
/// * `original_query`: The original unmodified query string
/// * `offset`: Result offset used in the query
/// * `length`: Number of results returned
/// * `count`: Number of results in this response
/// * `count_total`: Total number of matching documents
/// * `query_terms`: Parsed query terms
/// * `results`: Search results as JSON string
/// * `suggestions`: Query suggestions (spelling corrections, etc.)
#[pyclass(name = "SearchResultObject")]
pub struct PySearchResultObject {
    pub inner: SearchResultObject,
}

#[pymethods]
impl PySearchResultObject {
    /// Get the query execution time in microseconds
    #[getter]
    fn time(&self) -> u128 {
        self.inner.time
    }

    /// Get the processed query string
    #[getter]
    fn query(&self) -> String {
        self.inner.query.clone()
    }

    /// Get the original unmodified query string
    #[getter]
    fn original_query(&self) -> String {
        self.inner.original_query.clone()
    }

    /// Get the result offset used in the query
    #[getter]
    fn offset(&self) -> usize {
        self.inner.offset
    }

    /// Get the number of results returned in this response
    #[getter]
    fn length(&self) -> usize {
        self.inner.length
    }

    /// Get the number of results in this response
    #[getter]
    fn count(&self) -> usize {
        self.inner.count
    }

    /// Get the total number of matching documents
    #[getter]
    fn count_total(&self) -> usize {
        self.inner.count_total
    }

    /// Get the parsed query terms
    #[getter]
    fn query_terms(&self) -> Vec<String> {
        self.inner.query_terms.clone()
    }

    /// Get the search results as a JSON string
    #[getter]
    fn results(&self) -> String {
        serde_json::to_string(&self.inner.results).unwrap_or_default()
    }

    /// Get query suggestions (spelling corrections, etc.)
    #[getter]
    fn suggestions(&self) -> Vec<String> {
        self.inner.suggestions.clone()
    }
}

/// Python wrapper for GetIteratorRequest
///
/// Specifies parameters for iterating over documents in an index.
///
/// Properties:
/// * `document_id`: Base document ID to start from (None to start from beginning/end)
/// * `skip`: Number of documents to skip from the starting point
/// * `take`: Number of documents to return (positive = forward, negative = backward)
/// * `include_deleted`: Include deleted documents in results
/// * `include_document`: Retrieve full documents along with IDs
/// * `fields`: Which fields to return (empty = all stored fields)
///
/// # Examples
///
/// ```python
/// from seekstorm_client_py import GetIteratorRequest
///
/// # Create an iterator request
/// request = GetIteratorRequest()
/// request.take = 10
/// request.include_document = True
/// request.fields = ["title", "body"]
/// ```
#[pyclass(name = "GetIteratorRequest")]
pub struct PyGetIteratorRequest {
    pub inner: GetIteratorRequest,
}

#[pymethods]
impl PyGetIteratorRequest {
    /// Create a new GetIteratorRequest with default settings
    #[new]
    #[pyo3(signature = (), text_signature = "()")]
    fn new() -> Self {
        PyGetIteratorRequest {
            inner: GetIteratorRequest {
                document_id: None,
                skip: 0,
                take: 1,
                include_deleted: false,
                include_document: false,
                fields: Vec::new(),
            },
        }
    }

    /// Get the base document ID to start from
    #[getter]
    fn document_id(&self) -> Option<u64> {
        self.inner.document_id
    }

    /// Set the base document ID to start from
    #[setter]
    fn set_document_id(&mut self, value: Option<u64>) {
        self.inner.document_id = value;
    }

    /// Get the number of documents to skip
    #[getter]
    fn skip(&self) -> usize {
        self.inner.skip
    }

    /// Set the number of documents to skip
    #[setter]
    fn set_skip(&mut self, value: usize) {
        self.inner.skip = value;
    }

    /// Get the number of documents to return
    #[getter]
    fn take(&self) -> isize {
        self.inner.take
    }

    /// Set the number of documents to return
    #[setter]
    fn set_take(&mut self, value: isize) {
        self.inner.take = value;
    }

    /// Get whether deleted documents are included
    #[getter]
    fn include_deleted(&self) -> bool {
        self.inner.include_deleted
    }

    /// Set whether to include deleted documents
    #[setter]
    fn set_include_deleted(&mut self, value: bool) {
        self.inner.include_deleted = value;
    }

    /// Get whether documents are retrieved along with IDs
    #[getter]
    fn include_document(&self) -> bool {
        self.inner.include_document
    }

    /// Set whether to retrieve documents
    #[setter]
    fn set_include_document(&mut self, value: bool) {
        self.inner.include_document = value;
    }

    /// Get the list of fields to return
    #[getter]
    fn fields(&self) -> Vec<String> {
        self.inner.fields.clone()
    }

    /// Set the list of fields to return
    #[setter]
    fn set_fields(&mut self, value: Vec<String>) {
        self.inner.fields = value;
    }
}

/// Python wrapper for a single document iterator result item
///
/// Represents a single document returned by the iterator.
///
/// Properties:
/// * `doc_id`: The document ID
/// * `doc`: The document content (if requested), as JSON string
#[pyclass(name = "IteratorResultItem")]
pub struct PyIteratorResultItem {
    pub inner: IteratorResultItem,
}

#[pymethods]
impl PyIteratorResultItem {
    /// Get the document ID
    #[getter]
    fn doc_id(&self) -> u64 {
        self.inner.doc_id
    }

    /// Get the document content as JSON string (if included)
    #[getter]
    fn doc(&self) -> Option<String> {
        self.inner
            .doc
            .as_ref()
            .map(|d| serde_json::to_string(d).unwrap_or_default())
    }
}

/// Python wrapper for IteratorResult
///
/// Contains the results of a document iterator operation.
///
/// Properties:
/// * `skip`: Number of documents actually skipped
/// * `results`: List of IteratorResultItem objects
///
/// # Examples
///
/// ```python
/// from seekstorm_client_py import SeekStormClient, GetIteratorRequest
///
/// client = SeekStormClient()
/// request = GetIteratorRequest()
/// request.take = 10
/// request.include_document = True
///
/// result = client.document_iterator("http://127.0.0.1:80", "api-key", 1, request)
/// for item in result.results:
///     print(f"Document ID: {item.doc_id}")
/// ```
#[pyclass(name = "IteratorResult")]
pub struct PyIteratorResult {
    pub inner: IteratorResult,
}

#[pymethods]
impl PyIteratorResult {
    /// Get the number of documents actually skipped
    #[getter]
    fn skip(&self) -> usize {
        self.inner.skip
    }

    /// Get the list of iterator result items
    #[getter]
    fn results(&self) -> Vec<PyIteratorResultItem> {
        self.inner
            .results
            .iter()
            .map(|item| PyIteratorResultItem {
                inner: item.clone(),
            })
            .collect()
    }
}

/// Python wrapper for GetDocumentRequest
///
/// Specifies parameters for retrieving a document by ID.
///
/// Properties:
/// * `query_terms`: Terms to highlight in the document
/// * `highlights`: Which fields to create highlights/snippets for
/// * `fields`: Which fields to return
/// * `distance_fields`: Distance fields to calculate and return
///
/// # Examples
///
/// ```python
/// from seekstorm_client_py import GetDocumentRequest
///
/// request = GetDocumentRequest()
/// request.query_terms = ["search", "term"]
/// request.fields = ["title", "body"]
/// ```
#[pyclass(name = "GetDocumentRequest")]
pub struct PyGetDocumentRequest {
    pub inner: GetDocumentRequest,
}

#[pymethods]
impl PyGetDocumentRequest {
    /// Create a new GetDocumentRequest with default settings
    #[new]
    #[pyo3(signature = (), text_signature = "()")]
    fn new() -> Self {
        PyGetDocumentRequest {
            inner: GetDocumentRequest {
                query_terms: Vec::new(),
                highlights: Vec::new(),
                fields: Vec::new(),
                distance_fields: Vec::new(),
            },
        }
    }

    /// Get the query terms for highlighting
    #[getter]
    fn query_terms(&self) -> Vec<String> {
        self.inner.query_terms.clone()
    }

    /// Set the query terms for highlighting
    #[setter]
    fn set_query_terms(&mut self, value: Vec<String>) {
        self.inner.query_terms = value;
    }

    /// Get the fields to create highlights in
    #[getter]
    fn fields(&self) -> Vec<String> {
        self.inner.fields.clone()
    }

    /// Set the fields to return
    #[setter]
    fn set_fields(&mut self, value: Vec<String>) {
        self.inner.fields = value;
    }

    /// Get highlights as JSON string.
    #[getter]
    fn highlights(&self) -> PyResult<String> {
        to_json_string(&self.inner.highlights)
    }

    /// Set highlights from JSON string.
    #[setter]
    fn set_highlights(&mut self, value: String) -> PyResult<()> {
        self.inner.highlights = from_json_str(&value, "highlights")?;
        Ok(())
    }

    /// Get distance_fields as JSON string.
    #[getter]
    fn distance_fields(&self) -> PyResult<String> {
        to_json_string(&self.inner.distance_fields)
    }

    /// Set distance_fields from JSON string.
    #[setter]
    fn set_distance_fields(&mut self, value: String) -> PyResult<()> {
        self.inner.distance_fields = from_json_str(&value, "distance_fields")?;
        Ok(())
    }
}

/// Python wrapper for ApikeyQuotaObject
///
/// Specifies quota and rate limit settings for an API key.
///
/// Properties:
/// * `indices_max`: Maximum number of indices per API key
/// * `indices_size_max`: Maximum combined index size in bytes
/// * `documents_max`: Maximum combined number of documents across all indices
/// * `operations_max`: Maximum operations per month (index/update/delete/query)
/// * `rate_limit`: Maximum queries per second
/// * `demo`: Create a fixed demo API key (default: false)
///
/// # Examples
///
/// ```python
/// from seekstorm_client_py import ApikeyQuotaObject
///
/// quota = ApikeyQuotaObject()
/// quota.indices_max = 10
/// quota.indices_size_max = 1000
/// quota.documents_max = 1000000
/// quota.operations_max = 100000
/// quota.rate_limit = 100
/// quota.demo = False
/// ```
///
/// Notes:
/// * `rate_limit = None` disables explicit rate limiting in the request payload.
#[pyclass(name = "ApikeyQuotaObject")]
pub struct PyApikeyQuotaObject {
    pub inner: ApikeyQuotaObject,
}

#[pymethods]
impl PyApikeyQuotaObject {
    /// Create a new ApikeyQuotaObject with default settings
    ///
    /// Returns:
    /// * `ApikeyQuotaObject`: New quota object with server-side defaults.
    ///
    /// Example:
    /// ```python
    /// quota = ApikeyQuotaObject()
    /// quota.indices_max = 10
    /// quota.rate_limit = None
    /// ```
    #[new]
    #[pyo3(signature = (), text_signature = "()")]
    fn new() -> Self {
        PyApikeyQuotaObject {
            inner: ApikeyQuotaObject::default(),
        }
    }

    /// Get the maximum number of indices.
    #[getter]
    fn indices_max(&self) -> usize {
        self.inner.indices_max
    }

    /// Set the maximum number of indices.
    ///
    /// Arguments:
    /// * `value`: Upper bound for index count.
    #[setter]
    fn set_indices_max(&mut self, value: usize) {
        self.inner.indices_max = value;
    }

    /// Get the maximum combined index size in bytes.
    #[getter]
    fn indices_size_max(&self) -> usize {
        self.inner.indices_size_max
    }

    /// Set the maximum combined index size in bytes.
    ///
    /// Arguments:
    /// * `value`: Combined storage quota across all indices.
    #[setter]
    fn set_indices_size_max(&mut self, value: usize) {
        self.inner.indices_size_max = value;
    }

    /// Get the maximum number of documents.
    #[getter]
    fn documents_max(&self) -> usize {
        self.inner.documents_max
    }

    /// Set the maximum number of documents.
    ///
    /// Arguments:
    /// * `value`: Document-count quota across all indices.
    #[setter]
    fn set_documents_max(&mut self, value: usize) {
        self.inner.documents_max = value;
    }

    /// Get the maximum operations per month.
    #[getter]
    fn operations_max(&self) -> usize {
        self.inner.operations_max
    }

    /// Set the maximum operations per month.
    ///
    /// Arguments:
    /// * `value`: Monthly operation quota.
    #[setter]
    fn set_operations_max(&mut self, value: usize) {
        self.inner.operations_max = value;
    }

    /// Get the optional rate limit (queries per second).
    #[getter]
    fn rate_limit(&self) -> Option<usize> {
        self.inner.rate_limit
    }

    /// Set the optional rate limit (queries per second).
    ///
    /// Arguments:
    /// * `value`: Maximum query rate, or `None` for no explicit limit.
    ///
    /// Example:
    /// ```python
    /// quota.rate_limit = 100
    /// # or disable limit
    /// quota.rate_limit = None
    /// ```
    #[setter]
    fn set_rate_limit(&mut self, value: Option<usize>) {
        self.inner.rate_limit = value;
    }

    /// Get whether demo restrictions are enabled.
    #[getter]
    fn demo(&self) -> bool {
        self.inner.demo
    }

    /// Set whether demo restrictions are enabled.
    ///
    /// Arguments:
    /// * `value`: `true` for demo key behavior, `false` for standard key behavior.
    #[setter]
    fn set_demo(&mut self, value: bool) {
        self.inner.demo = value;
    }
}

/// Python wrapper for CreateIndexRequest
///
/// Specifies configuration for creating a new search index.
///
/// Properties:
/// * `index_name`: Name of the index (informational)
/// * `similarity`: Lexical similarity function (Bm25f or Bm25fProximity)
/// * `tokenizer`: Tokenizer type (UnicodeAlphanumeric, etc.)
/// * `stemmer`: Stemming type (default: None)
/// * `stop_words`: Stop word filtering (default: None)
/// * `frequent_words`: Frequent word optimization (default: None)
/// * `ngram_indexing`: N-gram indexing flags (0 = disabled)
/// * `document_compression`: Compression type (Snappy, etc.)
/// * `schema`: Field schema as JSON via property setter/getter
/// * `stop_words`, `frequent_words`: configurable via JSON property setters/getters
/// * `synonyms`, `spelling_correction`, `query_completion`, `clustering`, `inference`:
///   configurable via JSON property setters/getters
///
/// # Examples
///
/// ```python
/// from seekstorm_client_py import CreateIndexRequest
///
/// request = CreateIndexRequest()
/// request.index_name = "my_index"
/// request.similarity = "Bm25fProximity"
/// request.tokenizer = "UnicodeAlphanumeric"
///
/// # Set complex nested fields with JSON
/// request.set_schema_json('[{"field":"title",...}]')
/// ```
///
/// Valid enum strings:
/// * `similarity`: `"Bm25f"`, `"Bm25fProximity"`
/// * `tokenizer`: `"UnicodeAlphanumeric"`, `"UnicodeAlphanumericFolded"`,
///   `"AsciiAlphabetic"`, `"UnicodeAlphanumericZH"`
/// * `stemmer`: `"None"`, `"English"`, `"German"`
/// * `document_compression`: `"None"`, `"Lz4"`, `"Snappy"`, `"Zstd"`
#[pyclass(name = "CreateIndexRequest")]
pub struct PyCreateIndexRequest {
    pub inner: CreateIndexRequest,
}

#[pymethods]
impl PyCreateIndexRequest {
    /// Create a new CreateIndexRequest with default settings
    ///
    /// Returns:
    /// * `CreateIndexRequest`: New request object with sensible defaults.
    ///
    /// Example:
    /// ```python
    /// req = CreateIndexRequest()
    /// req.index_name = "products"
    /// req.similarity = "Bm25f"
    /// req.tokenizer = "UnicodeAlphanumeric"
    /// ```
    #[new]
    #[pyo3(signature = (), text_signature = "()")]
    fn new() -> Self {
        PyCreateIndexRequest {
            inner: CreateIndexRequest {
                index_name: String::new(),
                schema: Vec::new(),
                similarity: Default::default(),
                tokenizer: Default::default(),
                stemmer: Default::default(),
                stop_words: Default::default(),
                frequent_words: Default::default(),
                ngram_indexing: 0,
                document_compression: DocumentCompression::None,
                synonyms: Vec::new(),
                spelling_correction: None,
                query_completion: None,
                clustering: Default::default(),
                inference: Default::default(),
            },
        }
    }

    /// Get the index name.
    #[getter]
    fn index_name(&self) -> String {
        self.inner.index_name.clone()
    }

    /// Set the index name.
    ///
    /// Arguments:
    /// * `value`: Human-readable index name.
    #[setter]
    fn set_index_name(&mut self, value: String) {
        self.inner.index_name = value;
    }

    /// Get the schema as JSON string.
    ///
    /// Returns:
    /// * `str`: JSON array containing schema field objects.
    #[getter]
    fn schema(&self) -> String {
        serde_json::to_string(&self.inner.schema).unwrap_or_default()
    }

    /// Set the schema from JSON string.
    ///
    /// Arguments:
    /// * `schema_json`: JSON array of field definitions.
    ///
    /// Raises:
    /// * `RuntimeError`: If parsing fails.
    ///
    /// Example:
    /// ```python
    /// req.schema = '[{"field":"title","field_type":"Text","store":true,"index_lexical":true}]'
    /// ```
    #[setter]
    fn set_schema(&mut self, schema_json: String) -> PyResult<()> {
        self.inner.schema = serde_json::from_str(&schema_json)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to parse schema JSON: {}", e)))?;
        Ok(())
    }

    /// Set the schema from JSON string
    ///
    /// Arguments:
    /// * `schema_json`: Array of SchemaField objects as JSON string
    ///
    /// Raises:
    /// * `RuntimeError`: If the JSON cannot be parsed into a valid schema definition.
    #[pyo3(text_signature = "(self, schema_json)")]
    fn set_schema_json(&mut self, schema_json: &str) -> PyResult<()> {
        self.inner.schema = serde_json::from_str(schema_json)
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to parse schema JSON: {}", e)))?;
        Ok(())
    }

    /// Get the similarity type.
    #[getter]
    fn similarity(&self) -> String {
        format!("{:?}", self.inner.similarity)
    }

    /// Set the similarity type.
    ///
    /// Allowed values:
    /// * `"Bm25f"`
    /// * `"Bm25fProximity"`
    ///
    /// Raises:
    /// * `RuntimeError`: If `value` is not one of the supported strings.
    #[setter]
    fn set_similarity(&mut self, value: String) -> PyResult<()> {
        self.inner.similarity = match value.as_str() {
            "Bm25f" => LexicalSimilarity::Bm25f,
            "Bm25fProximity" => LexicalSimilarity::Bm25fProximity,
            _ => return Err(PyRuntimeError::new_err("Invalid similarity type")),
        };
        Ok(())
    }

    /// Get the tokenizer type.
    #[getter]
    fn tokenizer(&self) -> String {
        format!("{:?}", self.inner.tokenizer)
    }

    /// Set the tokenizer type.
    ///
    /// Allowed values:
    /// * `"UnicodeAlphanumeric"`
    /// * `"UnicodeAlphanumericFolded"`
    /// * `"AsciiAlphabetic"`
    /// * `"UnicodeAlphanumericZH"`
    ///
    /// Raises:
    /// * `RuntimeError`: If `value` is not one of the supported strings.
    #[setter]
    fn set_tokenizer(&mut self, value: String) -> PyResult<()> {
        self.inner.tokenizer = match value.as_str() {
            "UnicodeAlphanumeric" => TokenizerType::UnicodeAlphanumeric,
            "UnicodeAlphanumericFolded" => TokenizerType::UnicodeAlphanumericFolded,
            "AsciiAlphabetic" => TokenizerType::AsciiAlphabetic,
            "UnicodeAlphanumericZH" => TokenizerType::UnicodeAlphanumericZH,
            _ => return Err(PyRuntimeError::new_err("Invalid tokenizer type")),
        };
        Ok(())
    }

    /// Get the stemmer type.
    #[getter]
    fn stemmer(&self) -> String {
        format!("{:?}", self.inner.stemmer)
    }

    /// Set the stemmer type.
    ///
    /// Allowed values:
    /// * `"None"`
    /// * `"English"`
    /// * `"German"`
    ///
    /// Raises:
    /// * `RuntimeError`: If `value` is not one of the supported strings.
    #[setter]
    fn set_stemmer(&mut self, value: String) -> PyResult<()> {
        self.inner.stemmer = match value.as_str() {
            "None" => StemmerType::None,
            "English" => StemmerType::English,
            "German" => StemmerType::German,
            _ => return Err(PyRuntimeError::new_err("Invalid stemmer type")),
        };
        Ok(())
    }

    /// Get stop_words as JSON string.
    #[getter]
    fn stop_words(&self) -> PyResult<String> {
        to_json_string(&self.inner.stop_words)
    }

    /// Set stop_words from JSON string.
    #[setter]
    fn set_stop_words(&mut self, value: String) -> PyResult<()> {
        self.inner.stop_words = from_json_str(&value, "stop_words")?;
        Ok(())
    }

    /// Get frequent_words as JSON string.
    #[getter]
    fn frequent_words(&self) -> PyResult<String> {
        to_json_string(&self.inner.frequent_words)
    }

    /// Set frequent_words from JSON string.
    #[setter]
    fn set_frequent_words(&mut self, value: String) -> PyResult<()> {
        self.inner.frequent_words = from_json_str(&value, "frequent_words")?;
        Ok(())
    }

    /// Get the n-gram indexing bit flags.
    #[getter]
    fn ngram_indexing(&self) -> u8 {
        self.inner.ngram_indexing
    }

    /// Set n-gram indexing bit flags.
    ///
    /// Arguments:
    /// * `value`: Bitwise combination of `NgramSet` values.
    #[setter]
    fn set_ngram_indexing(&mut self, value: u8) {
        self.inner.ngram_indexing = value;
    }

    /// Get the document compression type.
    #[getter]
    fn document_compression(&self) -> String {
        format!("{:?}", self.inner.document_compression)
    }

    /// Set the document compression type.
    ///
    /// Allowed values:
    /// * `"None"`
    /// * `"Lz4"`
    /// * `"Snappy"`
    /// * `"Zstd"`
    ///
    /// Raises:
    /// * `RuntimeError`: If `value` is not one of the supported strings.
    ///
    /// Example:
    /// ```python
    /// req.document_compression = "Snappy"
    /// ```
    #[setter]
    fn set_document_compression(&mut self, value: String) -> PyResult<()> {
        self.inner.document_compression = match value.as_str() {
            "None" => DocumentCompression::None,
            "Lz4" => DocumentCompression::Lz4,
            "Snappy" => DocumentCompression::Snappy,
            "Zstd" => DocumentCompression::Zstd,
            _ => return Err(PyRuntimeError::new_err("Invalid compression type")),
        };
        Ok(())
    }

    /// Set synonyms from JSON string
    ///
    /// Arguments:
    /// * `synonyms_json`: Array of synonym objects as JSON string
    ///
    /// Raises:
    /// * `RuntimeError`: If the JSON cannot be parsed into the expected synonym format.
    #[pyo3(text_signature = "(self, synonyms_json)")]
    fn set_synonyms_json(&mut self, synonyms_json: &str) -> PyResult<()> {
        self.inner.synonyms = from_json_str(synonyms_json, "synonyms")?;
        Ok(())
    }

    /// Get synonyms as JSON string.
    #[getter]
    fn synonyms(&self) -> PyResult<String> {
        to_json_string(&self.inner.synonyms)
    }

    /// Set synonyms from JSON string.
    #[setter]
    fn set_synonyms(&mut self, value: String) -> PyResult<()> {
        self.inner.synonyms = from_json_str(&value, "synonyms")?;
        Ok(())
    }

    /// Set spelling correction settings from JSON string
    ///
    /// Arguments:
    /// * `spelling_json`: SpellingCorrection object as JSON string
    ///
    /// Raises:
    /// * `RuntimeError`: If the JSON cannot be parsed into a valid spelling correction object.
    #[pyo3(text_signature = "(self, spelling_json)")]
    fn set_spelling_correction_json(&mut self, spelling_json: &str) -> PyResult<()> {
        self.inner.spelling_correction = from_json_str(spelling_json, "spelling_correction")?;
        Ok(())
    }

    /// Get spelling_correction as JSON string.
    #[getter]
    fn spelling_correction(&self) -> PyResult<String> {
        to_json_string(&self.inner.spelling_correction)
    }

    /// Set spelling_correction from JSON string.
    #[setter]
    fn set_spelling_correction(&mut self, value: String) -> PyResult<()> {
        self.inner.spelling_correction = from_json_str(&value, "spelling_correction")?;
        Ok(())
    }

    /// Set query completion settings from JSON string
    ///
    /// Arguments:
    /// * `completion_json`: QueryCompletion object as JSON string
    ///
    /// Raises:
    /// * `RuntimeError`: If the JSON cannot be parsed into a valid query completion object.
    #[pyo3(text_signature = "(self, completion_json)")]
    fn set_query_completion_json(&mut self, completion_json: &str) -> PyResult<()> {
        self.inner.query_completion = from_json_str(completion_json, "query_completion")?;
        Ok(())
    }

    /// Get query_completion as JSON string.
    #[getter]
    fn query_completion(&self) -> PyResult<String> {
        to_json_string(&self.inner.query_completion)
    }

    /// Set query_completion from JSON string.
    #[setter]
    fn set_query_completion(&mut self, value: String) -> PyResult<()> {
        self.inner.query_completion = from_json_str(&value, "query_completion")?;
        Ok(())
    }

    /// Get clustering as JSON string.
    #[getter]
    fn clustering(&self) -> PyResult<String> {
        to_json_string(&self.inner.clustering)
    }

    /// Set clustering from JSON string.
    #[setter]
    fn set_clustering(&mut self, value: String) -> PyResult<()> {
        self.inner.clustering = from_json_str(&value, "clustering")?;
        Ok(())
    }

    /// Get inference as JSON string.
    #[getter]
    fn inference(&self) -> PyResult<String> {
        to_json_string(&self.inner.inference)
    }

    /// Set inference from JSON string.
    #[setter]
    fn set_inference(&mut self, value: String) -> PyResult<()> {
        self.inner.inference = from_json_str(&value, "inference")?;
        Ok(())
    }
}

/// SeekStorm REST client for interacting with a SeekStorm server
///
/// Provides access to all SeekStorm REST API endpoints for managing indices,
/// documents, API keys, and executing searches.
///
/// # Examples
///
/// ```python
/// from seekstorm_client_py import SeekStormClient, SearchRequestObject
///
/// client = SeekStormClient()
/// base_url = "http://127.0.0.1:80"
/// api_key = "your-api-key-here"
///
/// # Check server status
/// status = client.live(base_url)
///
/// # Create a search request
/// search = SearchRequestObject("example search")
/// search.offset = 0
/// search.length = 10
///
/// # Execute search
/// results = client.query_index(base_url, api_key, 1, search)
/// print(f"Found {results.count_total} results in {results.time} microseconds")
/// ```
#[pyclass]
pub struct PySeekStormClient {
    inner: RestClient,
}

#[pymethods]
impl PySeekStormClient {
    /// Create a new SeekStormClient instance
    ///
    /// # Examples
    ///
    /// ```python
    /// from seekstorm_client_py import SeekStormClient
    /// client = SeekStormClient()
    /// ```
    #[new]
    #[pyo3(signature = (), text_signature = "()")]
    fn new() -> Self {
        PySeekStormClient {
            inner: RestClient::new(),
        }
    }

    /// Check if the SeekStorm server is live and responsive
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server (e.g., "http://127.0.0.1:80")
    ///
    /// Returns:
    /// * Server status message
    ///
    /// Raises:
    /// * `RuntimeError`: If the server cannot be reached or returns an error.
    ///
    /// Example:
    /// ```python
    /// client = SeekStormClient()
    /// status = client.live("http://127.0.0.1:80")
    /// ```
    #[pyo3(text_signature = "(self, base_url)")]
    fn live(&self, base_url: String) -> PyResult<String> {
        RUNTIME.with(|rt| {
            rt.block_on(async { self.inner.live(&base_url).await })
                .map_err(|e| PyRuntimeError::new_err(format!("SeekStorm Server Error: {:?}", e)))
        })
    }

    /// Index a single document
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index
    /// * `document_json`: Document data as JSON string
    ///
    /// Returns:
    /// * Number of documents indexed
    ///
    /// Raises:
    /// * `RuntimeError`: If JSON parsing fails, request transmission fails, or the server returns an error.
    ///
    /// Example:
    /// ```python
    /// count = client.index_document(
    ///     base_url,
    ///     api_key,
    ///     index_id,
    ///     '{"title":"Hello","body":"World"}'
    /// )
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id, document_json)")]
    fn index_document(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
        document_json: &str,
    ) -> PyResult<usize> {
        let document: serde_json::Value = serde_json::from_str(document_json).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to parse document JSON: {}", e))
        })?;

        let url = format!("{}/api/v1/index/{}/doc", base_url, index_id);

        RUNTIME.with(|rt| {
            rt.block_on(async {
                let response = self
                    .inner
                    .client
                    .post(&url)
                    .json(&document)
                    .header("apikey", apikey_base64)
                    .send()
                    .await
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to index document: {:?}", e)))?;

                let status = response.status();
                let body = response
                    .text()
                    .await
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to read index response: {:?}", e)))?;

                if status.is_success() {
                    parse_count_response(&body)
                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to parse response: {}", e)))
                } else {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to index document: {} {}",
                        status, body
                    )))
                }
            })
        })
    }

    /// Execute a search query against an index
    ///
    /// Executes a search query and returns matching documents with relevance scores.
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index to search
    /// * `request`: SearchRequestObject with query parameters
    ///
    /// Returns:
    /// * `SearchResultObject`: The search results
    ///
    /// Raises:
    /// * `RuntimeError`: If the request fails or the server returns an error.
    ///
    /// Example:
    /// ```python
    /// req = SearchRequestObject("+hello +world")
    /// req.offset = 0
    /// req.length = 10
    /// results = client.query_index(base_url, api_key, index_id, req)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id, request)")]
    fn query_index(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
        request: Bound<'_, PySearchRequestObject>,
    ) -> PyResult<PySearchResultObject> {
        // Extract all data before the blocking call to avoid Send issues with Python types
        let search_request = {
            let req_obj = request.borrow();
            req_obj.inner.clone()
        };

        RUNTIME.with(|rt| {
            let result = rt
                .block_on(async {
                    self.inner
                        .query_index(&base_url, &apikey_base64, index_id, search_request)
                        .await
                })
                .map_err(|e| PyRuntimeError::new_err(format!("Query failed: {:?}", e)))?;

            Ok(PySearchResultObject { inner: result })
        })
    }

    /// Create a new API key
    ///
    /// Creates a new API key with specified quota restrictions.
    /// Requires the Base64 encoded master API key.
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `master_apikey`: The Base64 encoded master API key
    /// * `quota`: ApikeyQuotaObject with quota and rate limit settings
    ///
    /// Returns:
    /// * New API key (Base64 encoded string)
    ///
    /// Raises:
    /// * `RuntimeError`: If the API key could not be created.
    ///
    /// Example:
    /// ```python
    /// quota = ApikeyQuotaObject()
    /// quota.indices_max = 10
    /// quota.documents_max = 1_000_000
    /// new_api_key = client.create_apikey(base_url, master_api_key, quota)
    /// ```
    #[pyo3(text_signature = "(self, base_url, master_apikey, quota)")]
    fn create_apikey(
        &self,
        base_url: String,
        master_apikey: String,
        quota: Bound<'_, PyApikeyQuotaObject>,
    ) -> PyResult<String> {
        let quota_obj = {
            let q = quota.borrow();
            q.inner.clone()
        };

        RUNTIME.with(|rt| {
            rt.block_on(async {
                self.inner
                    .create_apikey(&base_url, &master_apikey, &quota_obj)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Create API key failed: {:?}", e)))
        })
    }

    /// Delete an API key
    ///
    /// WARNING: This will delete all indices and documents associated with the API key.
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key to delete
    /// * `master_apikey_base64`: The Base64 encoded master API key
    ///
    /// Returns:
    /// * Number of remaining API keys
    ///
    /// Raises:
    /// * `RuntimeError`: If deletion fails.
    ///
    /// Example:
    /// ```python
    /// remaining = client.delete_apikey(base_url, api_key_to_delete, master_api_key)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, master_apikey_base64)")]
    fn delete_apikey(
        &self,
        base_url: String,
        apikey_base64: String,
        master_apikey_base64: String,
    ) -> PyResult<u64> {
        RUNTIME.with(|rt| {
            rt.block_on(async {
                self.inner
                    .delete_apikey(&base_url, &apikey_base64, &master_apikey_base64)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Delete API key failed: {:?}", e)))
        })
    }

    /// Get information about all indices associated with an API key
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    ///
    /// Returns:
    /// * JSON string containing IndexResponseObject array
    ///
    /// Raises:
    /// * `RuntimeError`: If the request fails or response serialization fails.
    ///
    /// Example:
    /// ```python
    /// info_json = client.get_apikey_info(base_url, api_key)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64)")]
    fn get_apikey_info(&self, base_url: String, apikey_base64: String) -> PyResult<String> {
        RUNTIME.with(|rt| {
            let result = rt
                .block_on(async { self.inner.get_apikey_info(&base_url, &apikey_base64).await })
                .map_err(|e| {
                    PyRuntimeError::new_err(format!("Get API key info failed: {:?}", e))
                })?;

            serde_json::to_string(&result)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize result: {}", e)))
        })
    }

    /// Create a new search index
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `request`: CreateIndexRequest with index configuration
    ///
    /// Returns:
    /// * Index ID (u64)
    ///
    /// Raises:
    /// * `RuntimeError`: If index creation fails.
    ///
    /// Example:
    /// ```python
    /// req = CreateIndexRequest()
    /// req.index_name = "docs"
    /// req.similarity = "Bm25f"
    /// req.tokenizer = "UnicodeAlphanumeric"
    /// req.set_schema_json('[{"field":"title","field_type":"Text","store":true,"index_lexical":true}]')
    /// index_id = client.create_index(base_url, api_key, req)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, request)")]
    fn create_index(
        &self,
        base_url: String,
        apikey_base64: String,
        request: Bound<'_, PyCreateIndexRequest>,
    ) -> PyResult<u64> {
        let index_request = {
            let req = request.borrow();
            req.inner.clone()
        };

        RUNTIME.with(|rt| {
            rt.block_on(async {
                self.inner
                    .create_index(&base_url, &apikey_base64, &index_request)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Create index failed: {:?}", e)))
        })
    }

    /// Delete an index and all its documents
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index to delete
    ///
    /// Returns:
    /// * Number of deleted indices
    ///
    /// Raises:
    /// * `RuntimeError`: If index deletion fails.
    ///
    /// Example:
    /// ```python
    /// remaining_indices = client.delete_index(base_url, api_key, index_id)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id)")]
    fn delete_index(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
    ) -> PyResult<u64> {
        RUNTIME.with(|rt| {
            rt.block_on(async {
                self.inner
                    .delete_index(&base_url, &apikey_base64, index_id)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Delete index failed: {:?}", e)))
        })
    }

    /// Clear all documents from an index while preserving the index structure
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index to clear
    ///
    /// Returns:
    /// * Number of documents deleted
    ///
    /// Raises:
    /// * `RuntimeError`: If clear operation fails.
    ///
    /// Example:
    /// ```python
    /// deleted_docs = client.clear_index(base_url, api_key, index_id)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id)")]
    fn clear_index(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
    ) -> PyResult<usize> {
        RUNTIME.with(|rt| {
            rt.block_on(async {
                self.inner
                    .clear_index(&base_url, &apikey_base64, index_id)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Clear index failed: {:?}", e)))
        })
    }

    /// Commit pending changes to an index
    ///
    /// Flushes buffered operations and writes changes to disk.
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index to commit
    ///
    /// Returns:
    /// * Number of committed documents
    ///
    /// Raises:
    /// * `RuntimeError`: If commit fails or response parsing fails.
    ///
    /// Example:
    /// ```python
    /// committed_docs = client.commit_index(base_url, api_key, index_id)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id)")]
    fn commit_index(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
    ) -> PyResult<u64> {
        let url = format!("{}/api/v1/index/{}", base_url, index_id);

        RUNTIME.with(|rt| {
            rt.block_on(async {
                let response = self
                    .inner
                    .client
                    .patch(&url)
                    .header("apikey", apikey_base64)
                    .send()
                    .await
                    .map_err(|e| PyRuntimeError::new_err(format!("Commit index failed: {:?}", e)))?;

                let status = response.status();
                let body = response
                    .text()
                    .await
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to read commit response: {:?}", e)))?;

                if status.is_success() {
                    parse_u64_response(&body)
                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to parse response: {}", e)))
                } else {
                    Err(PyRuntimeError::new_err(format!(
                        "Commit index failed: {} {}",
                        status, body
                    )))
                }
            })
        })
    }

    /// Get information about an index
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index
    ///
    /// Returns:
    /// * JSON string containing IndexResponseObject
    ///
    /// Raises:
    /// * `RuntimeError`: If the request fails or response serialization fails.
    ///
    /// Example:
    /// ```python
    /// index_info_json = client.get_index_info(base_url, api_key, index_id)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id)")]
    fn get_index_info(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
    ) -> PyResult<String> {
        RUNTIME.with(|rt| {
            let result = rt
                .block_on(async {
                    self.inner
                        .get_index_info(&base_url, &apikey_base64, index_id)
                        .await
                })
                .map_err(|e| PyRuntimeError::new_err(format!("Get index info failed: {:?}", e)))?;

            serde_json::to_string(&result)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize result: {}", e)))
        })
    }

    /// Index multiple documents in bulk
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index
    /// * `documents_json`: Array of Document objects as JSON string
    ///
    /// Returns:
    /// * Number of documents indexed
    ///
    /// Raises:
    /// * `RuntimeError`: If JSON parsing fails, request transmission fails, or the server returns an error.
    ///
    /// Example:
    /// ```python
    /// docs_json = '[{"title":"A"},{"title":"B"}]'
    /// count = client.index_documents(base_url, api_key, index_id, docs_json)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id, documents_json)")]
    fn index_documents(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
        documents_json: &str,
    ) -> PyResult<usize> {
        let documents: Vec<serde_json::Value> = serde_json::from_str(documents_json).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to parse documents JSON: {}", e))
        })?;

        let url = format!("{}/api/v1/index/{}/doc", base_url, index_id);

        RUNTIME.with(|rt| {
            rt.block_on(async {
                let response = self
                    .inner
                    .client
                    .post(&url)
                    .json(&documents)
                    .header("apikey", apikey_base64)
                    .send()
                    .await
                    .map_err(|e| PyRuntimeError::new_err(format!("Index documents failed: {:?}", e)))?;

                let status = response.status();
                let body = response
                    .text()
                    .await
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to read index response: {:?}", e)))?;

                if status.is_success() {
                    parse_count_response(&body)
                        .map_err(|e| PyRuntimeError::new_err(format!("Failed to parse response: {}", e)))
                } else {
                    Err(PyRuntimeError::new_err(format!(
                        "Index documents failed: {} {}",
                        status, body
                    )))
                }
            })
        })
    }

    /// Index a PDF file
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index
    /// * `file_path`: Path to the PDF file
    /// * `file_date`: File modification date as Unix timestamp
    /// * `document_bytes`: Raw PDF file bytes
    ///
    /// Returns:
    /// * Number of documents indexed from the PDF
    ///
    /// Raises:
    /// * `RuntimeError`: If the upload fails.
    ///
    /// Example:
    /// ```python
    /// with open("manual.pdf", "rb") as f:
    ///     content = f.read()
    /// indexed = client.index_pdf(base_url, api_key, index_id, "manual.pdf", 0, content)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id, file_path, file_date, document_bytes)")]
    fn index_pdf(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
        file_path: String,
        file_date: i64,
        document_bytes: Vec<u8>,
    ) -> PyResult<usize> {
        RUNTIME.with(|rt| {
            rt.block_on(async {
                self.inner
                    .index_pdf(
                        &base_url,
                        &apikey_base64,
                        index_id,
                        Path::new(&file_path),
                        file_date,
                        document_bytes,
                    )
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Index PDF failed: {:?}", e)))
        })
    }

    /// Retrieve a PDF file by document ID
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index
    /// * `doc_id`: The document ID of the PDF
    ///
    /// Returns:
    /// * PDF file bytes
    ///
    /// Raises:
    /// * `RuntimeError`: If retrieval fails.
    ///
    /// Example:
    /// ```python
    /// pdf_bytes = client.get_pdf(base_url, api_key, index_id, doc_id)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id, doc_id)")]
    fn get_pdf(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
        doc_id: u64,
    ) -> PyResult<Vec<u8>> {
        RUNTIME.with(|rt| {
            rt.block_on(async {
                self.inner
                    .get_pdf(&base_url, &apikey_base64, index_id, doc_id)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Get PDF failed: {:?}", e)))
        })
    }

    /// Retrieve a single document by ID
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index
    /// * `doc_id`: The document ID to retrieve
    /// * `request`: GetDocumentRequest with document retrieval parameters
    ///
    /// Returns:
    /// * JSON string containing Document object
    ///
    /// Raises:
    /// * `RuntimeError`: If retrieval or response serialization fails.
    ///
    /// Example:
    /// ```python
    /// req = GetDocumentRequest()
    /// req.fields = ["title", "body"]
    /// doc_json = client.get_document(base_url, api_key, index_id, doc_id, req)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id, doc_id, request)")]
    fn get_document(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
        doc_id: u64,
        request: Bound<'_, PyGetDocumentRequest>,
    ) -> PyResult<String> {
        let get_document_request = {
            let req_obj = request.borrow();
            req_obj.inner.clone()
        };

        RUNTIME.with(|rt| {
            let result = rt
                .block_on(async {
                    self.inner
                        .get_document(
                            &base_url,
                            &apikey_base64,
                            index_id,
                            doc_id,
                            &get_document_request,
                        )
                        .await
                })
                .map_err(|e| PyRuntimeError::new_err(format!("Get document failed: {:?}", e)))?;

            serde_json::to_string(&result)
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to serialize result: {}", e)))
        })
    }

    /// Update a single document by ID
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index
    /// * `doc_id`: The document ID to update
    /// * `document_json`: Updated Document data as JSON string
    ///
    /// Returns:
    /// * Number of documents updated
    ///
    /// Raises:
    /// * `RuntimeError`: If JSON parsing fails or update fails.
    ///
    /// Example:
    /// ```python
    /// updated = client.update_document(
    ///     base_url,
    ///     api_key,
    ///     index_id,
    ///     doc_id,
    ///     '{"title":"Updated title"}'
    /// )
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id, doc_id, document_json)")]
    fn update_document(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
        doc_id: u64,
        document_json: &str,
    ) -> PyResult<usize> {
        let document: Document = serde_json::from_str(document_json).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to parse document JSON: {}", e))
        })?;

        RUNTIME.with(|rt| {
            rt.block_on(async {
                self.inner
                    .update_document(&base_url, &apikey_base64, index_id, (doc_id, document))
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Update document failed: {:?}", e)))
        })
    }

    /// Update multiple documents in bulk
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index
    /// * `documents_json`: Array of [doc_id, Document] pairs as JSON string
    ///
    /// Returns:
    /// * Number of documents updated
    ///
    /// Raises:
    /// * `RuntimeError`: If JSON parsing fails or update fails.
    ///
    /// Example:
    /// ```python
    /// updates_json = '[[1,{"title":"A"}],[2,{"title":"B"}]]'
    /// updated = client.update_documents(base_url, api_key, index_id, updates_json)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id, documents_json)")]
    fn update_documents(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
        documents_json: &str,
    ) -> PyResult<usize> {
        let doc_pairs: Vec<(u64, Document)> =
            serde_json::from_str(documents_json).map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to parse documents JSON: {}", e))
            })?;

        RUNTIME.with(|rt| {
            rt.block_on(async {
                self.inner
                    .update_documents(&base_url, &apikey_base64, index_id, doc_pairs)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Update documents failed: {:?}", e)))
        })
    }

    /// Delete a single document by document ID
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index
    /// * `doc_id`: The document ID to delete
    ///
    /// Returns:
    /// * Number of documents deleted
    ///
    /// Raises:
    /// * `RuntimeError`: If deletion fails.
    ///
    /// Example:
    /// ```python
    /// deleted = client.delete_document_by_docid(base_url, api_key, index_id, doc_id)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id, doc_id)")]
    fn delete_document_by_docid(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
        doc_id: u64,
    ) -> PyResult<usize> {
        RUNTIME.with(|rt| {
            rt.block_on(async {
                self.inner
                    .delete_document_by_docid(&base_url, &apikey_base64, index_id, doc_id)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Delete document failed: {:?}", e)))
        })
    }

    /// Delete multiple documents by document IDs
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index
    /// * `doc_ids`: List of document IDs to delete
    ///
    /// Returns:
    /// * Number of documents deleted
    ///
    /// Raises:
    /// * `RuntimeError`: If deletion fails.
    ///
    /// Example:
    /// ```python
    /// deleted = client.delete_documents_by_docid(base_url, api_key, index_id, [1, 2, 3])
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id, doc_ids)")]
    fn delete_documents_by_docid(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
        doc_ids: Vec<u64>,
    ) -> PyResult<usize> {
        RUNTIME.with(|rt| {
            rt.block_on(async {
                self.inner
                    .delete_documents_by_docid(&base_url, &apikey_base64, index_id, doc_ids)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Delete documents failed: {:?}", e)))
        })
    }

    /// Delete all documents matching a search query
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index
    /// * `query`: SearchRequestObject with query parameters
    ///
    /// Returns:
    /// * Number of documents deleted
    ///
    /// Raises:
    /// * `RuntimeError`: If deletion request fails.
    ///
    /// Example:
    /// ```python
    /// q = SearchRequestObject("+obsolete")
    /// deleted = client.delete_documents_by_query(base_url, api_key, index_id, q)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id, query)")]
    fn delete_documents_by_query(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
        query: Bound<'_, PySearchRequestObject>,
    ) -> PyResult<usize> {
        let search_request = {
            let req_obj = query.borrow();
            req_obj.inner.clone()
        };

        RUNTIME.with(|rt| {
            rt.block_on(async {
                self.inner
                    .delete_documents_by_query(&base_url, &apikey_base64, index_id, &search_request)
                    .await
            })
            .map_err(|e| {
                PyRuntimeError::new_err(format!("Delete documents by query failed: {:?}", e))
            })
        })
    }

    /// Iterate over all documents in an index
    ///
    /// Arguments:
    /// * `base_url`: The base URL of the SeekStorm server
    /// * `apikey_base64`: The Base64 encoded API key
    /// * `index_id`: The ID of the index
    /// * `request`: GetIteratorRequest with iteration parameters
    ///
    /// Returns:
    /// * `IteratorResult`: The iterator results containing document IDs and optional document content
    ///
    /// Raises:
    /// * `RuntimeError`: If iteration request fails.
    ///
    /// Example:
    /// ```python
    /// req = GetIteratorRequest()
    /// req.take = 10
    /// req.include_document = True
    /// page = client.document_iterator(base_url, api_key, index_id, req)
    /// ```
    #[pyo3(text_signature = "(self, base_url, apikey_base64, index_id, request)")]
    fn document_iterator(
        &self,
        base_url: String,
        apikey_base64: String,
        index_id: u64,
        request: Bound<'_, PyGetIteratorRequest>,
    ) -> PyResult<PyIteratorResult> {
        let iterator_request = {
            let req_obj = request.borrow();
            req_obj.inner.clone()
        };

        RUNTIME.with(|rt| {
            let result = rt
                .block_on(async {
                    self.inner
                        .document_iterator(&base_url, &apikey_base64, index_id, iterator_request)
                        .await
                })
                .map_err(|e| {
                    PyRuntimeError::new_err(format!("Document iterator failed: {:?}", e))
                })?;

            Ok(PyIteratorResult { inner: result })
        })
    }
}

#[pymodule]
fn seekstorm_client_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySeekStormClient>()?;
    m.add_class::<PySearchRequestObject>()?;
    m.add_class::<PySearchResultObject>()?;
    m.add_class::<PyGetIteratorRequest>()?;
    m.add_class::<PyIteratorResultItem>()?;
    m.add_class::<PyIteratorResult>()?;
    m.add_class::<PyGetDocumentRequest>()?;
    m.add_class::<PyApikeyQuotaObject>()?;
    m.add_class::<PyCreateIndexRequest>()?;
    Ok(())
}
