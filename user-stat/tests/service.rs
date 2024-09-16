#![cfg(target_os = "macos")]

use std::{net::SocketAddr, time::Duration};

use anyhow::Result;
use chrono::Utc;
use crm_core::ConfigExt;
use futures::StreamExt;
use prost_types::Timestamp;
use tokio::time::sleep;
use tonic::transport::Server;
use user_stat::{
    pb::{user_stats_client::UserStatsClient, IdQuery, QueryRequestBuilder, TimeQuery},
    AppConfig, UserStatsService,
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

    let mut stream = client.query(query).await?.into_inner();

    while let Some(user) = stream.next().await {
        println!("{:?}", user?);
    }

    Ok(())
}

async fn start_server(port: u32) -> Result<SocketAddr> {
    let addr = format!("127.0.0.1:{}", port).parse()?;

    let svc = UserStatsService::new(AppConfig::load()?).await;

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
