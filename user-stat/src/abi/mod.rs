use async_stream::stream;
use chrono::{DateTime, TimeZone, Utc};
use clickhouse::{query::Query, sql::Identifier, Client, Row};
use futures::Stream;
use prost_types::Timestamp;
use serde::{Deserialize, Serialize};
use tonic::{Code, Response, Status};
use tracing::{debug, error, instrument};

use crate::{
    pb::{QueryRequest, QueryRequestBuilder, TimeQuery, User},
    ServiceResult, UserStatsService,
};

#[derive(Row, Serialize, Deserialize)]
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

impl UserStatsService {
    #[instrument(name = "query", skip_all)]
    pub async fn query(
        &self,
        request: QueryRequest,
    ) -> ServiceResult<impl Stream<Item = Result<User, Status>> + Send + 'static> {
        let mut cursor = request
            .to_query(&self.client)
            .fetch::<UserRow>()
            .inspect_err(|e| {
                error!("query error: {}", e);
            })
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(stream! {
            while let Some(row) = cursor.next().await.map_err(|e| Status::internal(e.to_string()))? {
                yield Ok(row.into());
            }
        }))
    }

    #[instrument(name = "raw-query", skip(self))]
    pub async fn raw_query(
        &self,
        _query: String,
    ) -> ServiceResult<impl Stream<Item = Result<User, Status>> + Send + 'static> {
        let mut cursor = self
            .client
            .query("SELECT ?fields FROM ? LIMIT ?")
            .bind(Identifier("user_stat"))
            .bind(50)
            .fetch::<UserRow>()
            .map_err(|e| Status::new(Code::Unknown, e.to_string()))?;

        Ok(Response::new(stream! {
            while let Some(row) = cursor.next().await.map_err(|e| Status::internal(e.to_string()))? {
                yield Ok(row.into());
            }
        }))
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

    #[instrument(name = "to-query", skip_all)]
    pub fn to_query(&self, client: &Client) -> Query {
        let mut sql = String::from("SELECT ?fields FROM ?");

        let time_query = self
            .timestamps
            .clone()
            .into_iter()
            .map(|(k, v)| timestamp_query(&k, v.lower, v.upper))
            .collect::<Vec<_>>();

        let id_query = self
            .ids
            .clone()
            .into_iter()
            .map(|(k, v)| ids_query(&k, v.ids))
            .collect::<Vec<_>>();

        sql.push_str(" WHERE ");

        let mut time_bind: Vec<i64> = vec![];
        let mut id_bind: Vec<u32> = vec![];

        sql.push_str(
            &time_query
                .into_iter()
                .map(|(condition, bind)| {
                    time_bind.extend(bind);
                    condition
                })
                .collect::<Vec<_>>()
                .join(" AND "),
        );

        sql.push_str(" AND ");

        sql.push_str(
            &id_query
                .into_iter()
                .map(|(condition, bind)| {
                    id_bind.extend(bind);
                    condition
                })
                .collect::<Vec<_>>()
                .join(" AND "),
        );

        sql.push_str(" ORDER BY (email, created_at)");

        debug!("query: {}", sql);

        let mut query = client.query(&sql).bind(Identifier("user_stat"));

        query = time_bind
            .into_iter()
            .fold(query, |query, bind| query.bind(bind));

        query = id_bind
            .into_iter()
            .fold(query, |query, bind| query.bind(bind));

        query
    }
}

fn ids_query(name: &str, ids: Vec<u32>) -> (String, Vec<u32>) {
    if ids.is_empty() {
        return ("TRUE".to_string(), vec![]);
    }

    (format!("arrayExists(x -> x IN (?), {name})"), ids)
}

fn timestamp_query(
    name: &str,
    lower: Option<Timestamp>,
    upper: Option<Timestamp>,
) -> (String, Vec<i64>) {
    if lower.is_none() && upper.is_none() {
        return ("TRUE".to_string(), vec![]);
    }

    if lower.is_none() {
        let upper = ts_to_utc(upper.unwrap());
        return (format!("{} <= ?", name), vec![upper.timestamp()]);
    }

    if upper.is_none() {
        let lower = ts_to_utc(lower.unwrap());
        return (format!("{} >= ?", name), vec![lower.timestamp()]);
    }

    (
        format!("{} BETWEEN ? AND ?", name),
        vec![
            ts_to_utc(lower.unwrap()).timestamp(),
            ts_to_utc(upper.unwrap()).timestamp(),
        ],
    )
}

fn ts_to_utc(ts: Timestamp) -> DateTime<Utc> {
    Utc.timestamp_opt(ts.seconds, ts.nanos as _).unwrap()
}
