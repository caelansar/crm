use anyhow::Result;
use async_stream::stream;
use chrono::{DateTime, TimeZone, Utc};
use clickhouse::{query::Query, sql::Identifier};
use futures::Stream;
use prost_types::Timestamp;
use tonic::{Code, Status};
use tracing::{debug, error, instrument};

use crate::pb::{QueryRequest, User};

use super::{Repo, UserRow};

#[derive(Clone)]
pub struct ClickHouseRepo {
    client: clickhouse::Client,
}

impl ClickHouseRepo {
    pub fn new(client: clickhouse::Client) -> Self {
        Self { client }
    }

    #[instrument(name = "to-query", skip_all)]
    pub fn to_query(&self, req: &QueryRequest) -> Query {
        let mut sql = String::from("SELECT ?fields FROM ?");

        let time_query = req
            .timestamps
            .iter()
            .map(|(k, v)| timestamp_query(k, v.lower.as_ref(), v.upper.as_ref()))
            .collect::<Vec<_>>();

        let id_query = req
            .ids
            .iter()
            .map(|(k, v)| ids_query(k, &v.ids))
            .collect::<Vec<_>>();

        sql.push_str(" WHERE ");

        let mut time_bind: Vec<i64> = vec![];
        let mut id_bind: Vec<u32> = vec![];

        sql.push_str(
            &time_query
                .into_iter()
                .map(|(condition, bind)| {
                    if let Some(bind) = bind {
                        time_bind.extend(bind);
                    }
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
                    if let Some(bind) = bind {
                        id_bind.extend(bind);
                    }
                    condition
                })
                .collect::<Vec<_>>()
                .join(" AND "),
        );

        sql.push_str(" ORDER BY (email, created_at)");

        debug!("query: {}", sql);

        let mut query = self
            .client
            .clone()
            .query(&sql)
            .bind(Identifier("user_stat"));

        query = time_bind
            .into_iter()
            .fold(query, |query, bind| query.bind(bind));

        query = id_bind
            .into_iter()
            .fold(query, |query, bind| query.bind(bind));

        query
    }
}

impl Repo for ClickHouseRepo {
    #[instrument(name = "query-clickhouse", skip_all)]
    async fn query(
        &self,
        request: QueryRequest,
    ) -> Result<impl Stream<Item = Result<User, Status>> + Send + 'static> {
        let mut cursor = self
            .to_query(&request)
            .fetch::<UserRow>()
            .inspect_err(|e| {
                error!("query error: {}", e);
            })
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(stream! {
            while let Some(row) = cursor.next().await.map_err(|e| Status::internal(e.to_string()))? {
                yield Ok(row.into());
            }
        })
    }

    #[instrument(name = "raw-query-clickhouse", skip(self))]
    async fn raw_query(
        &self,
        query: String,
    ) -> Result<impl Stream<Item = Result<User, Status>> + Send + 'static> {
        let mut cursor = self
            .client
            .query(&query)
            .fetch::<UserRow>()
            .map_err(|e| Status::new(Code::Unknown, e.to_string()))?;

        Ok(stream! {
            while let Some(row) = cursor.next().await.map_err(|e| Status::internal(e.to_string()))? {
                yield Ok(row.into());
            }
        })
    }
}

fn ids_query<'a>(name: &str, ids: &'a [u32]) -> (String, Option<&'a [u32]>) {
    if ids.is_empty() {
        return ("TRUE".to_string(), None);
    }

    (format!("arrayExists(x -> x IN (?), {name})"), Some(ids))
}

fn timestamp_query(
    name: &str,
    lower: Option<&Timestamp>,
    upper: Option<&Timestamp>,
) -> (String, Option<Vec<i64>>) {
    if lower.is_none() && upper.is_none() {
        return ("TRUE".to_string(), None);
    }

    if lower.is_none() {
        let upper = ts_to_utc(upper.unwrap());
        return (format!("{} <= ?", name), Some(vec![upper.timestamp()]));
    }

    if upper.is_none() {
        let lower = ts_to_utc(lower.unwrap());
        return (format!("{} >= ?", name), Some(vec![lower.timestamp()]));
    }

    (
        format!("{} BETWEEN ? AND ?", name),
        Some(vec![
            ts_to_utc(lower.unwrap()).timestamp(),
            ts_to_utc(upper.unwrap()).timestamp(),
        ]),
    )
}

#[inline(always)]
fn ts_to_utc(ts: &Timestamp) -> DateTime<Utc> {
    Utc.timestamp_opt(ts.seconds, ts.nanos as _).unwrap()
}
