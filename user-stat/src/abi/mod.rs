use std::fmt::Display;

use async_stream::stream;
use clickhouse::{sql::Identifier, Row};
use futures::Stream;
use serde::Deserialize;
use tonic::{Code, Response, Status};

use crate::{
    pb::{QueryRequest, User},
    ServiceResult, UserStatsServiceInner,
};

#[derive(Row, Deserialize)]
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

impl UserStatsServiceInner {
    pub async fn query(
        &self,
        request: QueryRequest,
    ) -> ServiceResult<impl Stream<Item = Result<User, Status>> + Send + 'static> {
        self.raw_query(request.to_string()).await
    }

    pub async fn raw_query(
        &self,
        _query: String,
    ) -> ServiceResult<impl Stream<Item = Result<User, Status>> + Send + 'static> {
        // TODO: do query
        let mut cursor = self
            .client
            .query("SELECT ?fields FROM ? WHERE ts BETWEEN ? AND ?")
            .bind(Identifier("users"))
            .bind(500)
            .bind(504)
            .fetch::<UserRow>()
            .map_err(|e| Status::new(Code::Unknown, e.to_string()))?;

        Ok(Response::new(stream! {
            while let Some(row) = cursor.next().await.map_err(|e| Status::new(Code::Unknown, e.to_string()))? {
                yield Ok(row.into());
            }
        }))
    }
}

impl Display for QueryRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: generate sql
        write!(f, "")
    }
}
