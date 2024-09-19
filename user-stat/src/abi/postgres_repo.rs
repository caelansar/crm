use anyhow::Result;
use futures::Stream;
use sqlx::PgPool;
use tonic::Status;

use super::{QueryRequest, Repo, User};

#[allow(unused)]
pub struct PostgresRepo {
    pool: PgPool,
}

#[allow(unused)]
impl PostgresRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl Repo for PostgresRepo {
    async fn query(
        &self,
        _request: QueryRequest,
    ) -> Result<impl Stream<Item = Result<User, Status>> + Send + 'static> {
        Ok(tokio_stream::iter(vec![Ok(User::default())]))
    }

    async fn raw_query(
        &self,
        _query: String,
    ) -> Result<impl Stream<Item = Result<User, Status>> + Send + 'static> {
        Ok(tokio_stream::iter(vec![Ok(User::default())]))
    }
}
