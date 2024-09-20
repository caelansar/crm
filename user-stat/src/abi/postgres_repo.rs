use anyhow::Result;
use chrono::{DateTime, TimeZone as _, Utc};
use futures::Stream;
use prost_types::Timestamp;
use sqlx::{PgPool, Postgres, QueryBuilder};
use tonic::Status;

use super::{QueryRequest, Repo, User, UserRow};

#[allow(unused)]
pub struct PostgresRepo {
    pool: PgPool,
}

#[allow(unused)]
impl PostgresRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn to_query(request: &QueryRequest) -> QueryBuilder<'_, Postgres> {
        let mut query_builder = QueryBuilder::new("SELECT email, name FROM user_stats WHERE ");

        let mut first_condition = true;

        for (k, v) in &request.timestamps {
            if !first_condition {
                query_builder.push(" AND ");
            }
            first_condition = false;

            if let Some(lower) = &v.lower {
                query_builder
                    .push(format!("{} >= ", k))
                    .push_bind(ts_to_utc(lower));
            }
            if let Some(upper) = &v.upper {
                if v.lower.is_some() {
                    query_builder.push(" AND ");
                }
                query_builder
                    .push(format!("{} <= ", k))
                    .push_bind(ts_to_utc(upper));
            }
        }

        for (k, v) in &request.ids {
            if !v.ids.is_empty() {
                if !first_condition {
                    query_builder.push(" AND ");
                }
                first_condition = false;

                query_builder.push("array[");
                let mut separated = query_builder.separated(", ");
                for id in v.ids.iter() {
                    separated.push_bind(*id as i32);
                }
                separated.push_unseparated(format!("] <@ {}", k));
            }
        }

        query_builder
    }
}

impl Repo for PostgresRepo {
    async fn query(
        &self,
        request: QueryRequest,
    ) -> Result<impl Stream<Item = Result<User, Status>> + Send + 'static> {
        let mut sql = PostgresRepo::to_query(&request);

        let ret = sql
            .build_query_as::<UserRow>()
            .fetch_all(&self.pool)
            .await?;

        Ok(futures::stream::iter(
            ret.into_iter().map(|row| Ok(row.into())),
        ))
    }

    async fn raw_query(
        &self,
        query: String,
    ) -> Result<impl Stream<Item = Result<User, Status>> + Send + 'static> {
        let ret = sqlx::query_as::<_, UserRow>(&query)
            .fetch_all(&self.pool)
            .await?;

        Ok(futures::stream::iter(
            ret.into_iter().map(|row| Ok(row.into())),
        ))
    }
}

fn ts_to_utc(ts: &Timestamp) -> DateTime<Utc> {
    Utc.timestamp_opt(ts.seconds, ts.nanos as _).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_request_to_string_should_work() {
        let d1 = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let d2 = Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap();
        let query = QueryRequest::new_with_dt("created_at", d1, d2);
        let sql = PostgresRepo::to_query(&query);
        assert_eq!(
            sql.sql().to_string(),
            "SELECT email, name FROM user_stats WHERE created_at >= $1 AND created_at <= $2"
        );
    }
}
