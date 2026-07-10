use crate::ocean_query::engine::QueryEngine;
use crate::ocean_query::error::QueryError;
use crate::ocean_query::types::{Query, QueryResult, RankedChunk};
use crate::ocean_vector::embedder::Embedder;

pub fn query(
    engine: &QueryEngine,
    q: Query,
    embedder: &dyn Embedder,
) -> Result<QueryResult, QueryError> {
    engine.query(q, embedder)
}

pub fn query_stream<'a>(
    engine: &'a QueryEngine,
    q: Query,
    embedder: &'a dyn Embedder,
) -> Result<impl Iterator<Item = Result<RankedChunk, QueryError>> + 'a, QueryError> {
    let result = engine.query(q, embedder)?;
    let iter = result.results.into_iter().map(Ok);
    Ok(iter)
}
