use std::{
    mem,
    net::SocketAddr,
    time::{Duration, Instant},
};

use anyhow::Result;
use chrono::Utc;
use clickhouse::{test, Client};
use crm_core::ConfigExt;
use futures::StreamExt;
use prost_types::Timestamp;
use tokio::time::sleep;
use tonic::transport::Server;
use user_stat::{
    pb::{user_stats_client::UserStatsClient, IdQuery, QueryRequestBuilder, TimeQuery, User},
    AppConfig, ClickHouseRepo, PostgresRepo, UserRow, UserStatsService,
};

const PORT_BASE: u32 = 6000;

#[tokio::test]
async fn query_should_work() -> Result<()> {
    let addr = start_server(PORT_BASE + 1).await?;
    let mut client = UserStatsClient::connect(format!("http://{addr}")).await?;
    let query = QueryRequestBuilder::default()
        .timestamp(("created_at".to_string(), tq(Some(700), None)))
        .timestamp(("last_visited_at".to_string(), tq(Some(700), Some(0))))
        .id(("finished".to_string(), id(&[404])))
        .build()
        .unwrap();

    let start = Instant::now();

    let stream = client.query(query).await?.into_inner();

    let users: Vec<User> = stream.filter_map(|r| async { r.ok() }).collect().await;

    println!("time elapsed: {:?}", start.elapsed());

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "test1");
    assert_eq!(users[0].email, "test1@example.com");
    assert_eq!(users[1].name, "test2");
    assert_eq!(users[1].email, "test2@example.com");

    Ok(())
}

#[cfg(feature = "test-util")]
mod postgres_test {
    use super::*;

    #[tokio::test]
    async fn query_should_work_postgres() -> Result<()> {
        let addr = start_server_postgres(PORT_BASE + 2).await?;

        let mut client = UserStatsClient::connect(format!("http://{addr}")).await?;
        let query = QueryRequestBuilder::default()
            .timestamp(("created_at".to_string(), tq(Some(700), None)))
            .timestamp(("last_visited_at".to_string(), tq(Some(700), Some(0))))
            .id(("finished".to_string(), id(&[404])))
            .build()
            .unwrap();

        let start = Instant::now();

        let stream = client.query(query).await?.into_inner();

        let users: Vec<User> = stream.filter_map(|r| async { r.ok() }).collect().await;

        println!("time elapsed: {:?}", start.elapsed());

        assert_eq!(users.len(), 2);
        assert_eq!(users[0].name, "Jane Smith");
        assert_eq!(users[0].email, "jane.smith@example.com");
        assert_eq!(users[1].name, "John Doe");
        assert_eq!(users[1].email, "john.doe@example.com");

        Ok(())
    }

    async fn start_server_postgres(port: u32) -> Result<SocketAddr> {
        use user_stat::tests::get_test_pool;

        let addr = format!("127.0.0.1:{}", port).parse()?;
        let (tdb, _pool) = get_test_pool(None).await;
        let repo = PostgresRepo::new(&tdb.url()).await?;

        mem::forget(tdb);

        let svc = UserStatsService::new_for_test(repo, AppConfig::load()?);

        tokio::spawn(async move {
            Server::builder()
                .add_service(svc.into_server())
                .serve(addr)
                .await
                .unwrap();
        });
        sleep(Duration::from_micros(1)).await;

        Ok(addr)
    }
}

async fn start_server(port: u32) -> Result<SocketAddr> {
    let addr = format!("127.0.0.1:{}", port).parse()?;

    let mock = test::Mock::new();

    let client = Client::default().with_url(mock.url());

    let repo = ClickHouseRepo::new(client);

    let list = vec![
        UserRow {
            name: "test1".to_string(),
            email: "test1@example.com".to_string(),
        },
        UserRow {
            name: "test2".to_string(),
            email: "test2@example.com".to_string(),
        },
    ];

    mock.add(test::handlers::provide(list));

    // Forget the mock instance to avoid it being dropped
    mem::forget(mock);

    let svc = UserStatsService::new_for_test(repo, AppConfig::load()?);

    tokio::spawn(async move {
        Server::builder()
            .add_service(svc.into_server())
            .serve(addr)
            .await
            .unwrap();
    });
    sleep(Duration::from_micros(1)).await;

    Ok(addr)
}

/// Create an id query with the given ids
fn id(id: &[u32]) -> IdQuery {
    IdQuery { ids: id.to_vec() }
}

/// Create a time query with the given lower and upper bounds
fn tq(lower: Option<i64>, upper: Option<i64>) -> TimeQuery {
    TimeQuery {
        lower: lower.map(to_ts),
        upper: upper.map(to_ts),
    }
}

/// Convert a number of days to a timestamp
fn to_ts(days: i64) -> Timestamp {
    let dt = Utc::now()
        .checked_sub_signed(chrono::Duration::days(days))
        .unwrap();
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}
