mod clickhouse_repo;
mod postgres_repo;

use anyhow::Result;
use chrono::{DateTime, Utc};
use clickhouse::Row;
use futures::Stream;
use prost_types::Timestamp;
use serde::{Deserialize, Serialize};
use tonic::Status;

use crate::pb::{QueryRequest, QueryRequestBuilder, TimeQuery, User};

pub use clickhouse_repo::ClickHouseRepo;
pub use postgres_repo::PostgresRepo;
#[derive(sqlx::FromRow, Row, Serialize, Deserialize)]
pub struct UserRow {
    pub name: String,
    pub email: String,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        User {
            name: row.name,
            email: row.email,
        }
    }
}

impl QueryRequest {
    pub fn new_with_dt(name: &str, lower: DateTime<Utc>, upper: DateTime<Utc>) -> Self {
        let ts = Timestamp {
            seconds: lower.timestamp(),
            nanos: 0,
        };
        let ts1 = Timestamp {
            seconds: upper.timestamp(),
            nanos: 0,
        };
        let tq = TimeQuery {
            lower: Some(ts),
            upper: Some(ts1),
        };

        QueryRequestBuilder::default()
            .timestamp((name.to_string(), tq))
            .build()
            .expect("Failed to build query request")
    }
}

/// A repository that can be used to query user stats
pub trait Repo: Send + Sync + 'static {
    fn query(
        &self,
        request: QueryRequest,
    ) -> impl std::future::Future<
        Output = Result<impl Stream<Item = Result<User, Status>> + Send + 'static>,
    > + Send;
    fn raw_query(
        &self,
        query: String,
    ) -> impl std::future::Future<
        Output = Result<impl Stream<Item = Result<User, Status>> + Send + 'static>,
    > + Send;
}
