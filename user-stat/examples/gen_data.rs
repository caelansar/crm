//! This example demonstrates how to generate and insert sample user statistics data into ClickHouse.
//!
//! It creates 500,000 fake user records with various attributes such as email, name, gender,
//! timestamps for different actions, and lists of viewed/started/finished content IDs.
//! These records are then inserted into ClickHouse in batches of 1,000, repeating 500 times
//!
//! This script is useful for:
//! - Testing the performance of ClickHouse with a large dataset
//! - Generating realistic-looking data for development and testing purposes
//! - Demonstrating how to use the `fake` crate to generate mock data
#![feature(duration_constructors)]

use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
};

use anyhow::Result;
use clickhouse::{Client, Row};
use fake::{
    faker::{internet::en::SafeEmail, name::en::Name, time::en::DateTimeBefore},
    Dummy, Fake, Faker,
};
use nanoid::nanoid;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use time::{Duration, OffsetDateTime};
use tokio::time::Instant;

#[derive(Debug, Clone, Dummy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
enum Gender {
    Female = 1,
    Male = 2,
    Unknown = 3,
}

#[derive(Debug, Clone, Row, Dummy, Serialize, Deserialize, PartialEq, Eq)]
struct UserStat {
    #[dummy(faker = "UniqueEmail")]
    email: String,
    #[dummy(faker = "Name()")]
    name: String,
    gender: Gender,
    #[serde(with = "clickhouse::serde::time::datetime")]
    #[dummy(faker = "DateTimeBefore(now())")]
    created_at: OffsetDateTime,
    #[serde(with = "clickhouse::serde::time::datetime::option")]
    #[dummy(faker = "DateTimeBefore(now())")]
    last_visited_at: Option<OffsetDateTime>,
    #[serde(with = "clickhouse::serde::time::datetime::option")]
    #[dummy(faker = "DateTimeBefore(now())")]
    last_watched_at: Option<OffsetDateTime>,
    #[dummy(faker = "IntList(5, 100000, 100000)")]
    recent_watched: Vec<i32>,
    #[dummy(faker = "IntList(5, 200000, 100000)")]
    viewed_but_not_started: Vec<i32>,
    #[dummy(faker = "IntList(5, 300000, 100000)")]
    started_but_not_finished: Vec<i32>,
    #[dummy(faker = "IntList(5, 400000, 100000)")]
    finished: Vec<i32>,
    #[serde(with = "clickhouse::serde::time::datetime::option")]
    #[dummy(faker = "DateTimeBefore(now())")]
    last_email_notification: Option<OffsetDateTime>,
    #[serde(with = "clickhouse::serde::time::datetime::option")]
    #[dummy(faker = "DateTimeBefore(now())")]
    last_in_app_notification: Option<OffsetDateTime>,
    #[serde(with = "clickhouse::serde::time::datetime::option")]
    #[dummy(faker = "DateTimeBefore(now())")]
    last_sms_notification: Option<OffsetDateTime>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::default()
        .with_url("http://localhost:18123")
        .with_database("crm");
    for i in 1..=500 {
        let users: HashSet<_> = (0..1000).map(|_| Faker.fake::<UserStat>()).collect();

        let start = Instant::now();
        bulk_insert(users, &client).await?;
        println!("Batch {} inserted in {:?}", i, start.elapsed());
    }
    Ok(())
}

async fn bulk_insert(users: HashSet<UserStat>, client: &Client) -> Result<()> {
    let mut inserter = client
        .inserter("user_stat")?
        .with_period(Some(std::time::Duration::from_secs(15)));

    for user in users {
        println!("{:?}", user);
        inserter.write(&user)?;
    }

    let stats = inserter.commit().await?;
    if stats.rows > 0 {
        println!(
            "{} bytes, {} rows, {} transactions have been inserted",
            stats.bytes, stats.rows, stats.transactions,
        );
    }

    inserter.end().await?;
    Ok(())
}

#[allow(unused)]
fn before(days: u64) -> OffsetDateTime {
    OffsetDateTime::now_utc()
        .checked_sub(Duration::days(-(days as i64)))
        .unwrap()
}

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

impl Hash for UserStat {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.email.hash(state);
    }
}

struct IntList(pub i32, pub i32, pub i32);

impl Dummy<IntList> for Vec<i32> {
    fn dummy_with_rng<R: Rng + ?Sized>(v: &IntList, rng: &mut R) -> Vec<i32> {
        let (max, start, len) = (v.0, v.1, v.2);
        let size = rng.gen_range(0..max);
        (0..size)
            .map(|_| rng.gen_range(start..start + len))
            .collect()
    }
}

struct UniqueEmail;
const ALPHABET: [char; 36] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

impl Dummy<UniqueEmail> for String {
    fn dummy_with_rng<R: Rng + ?Sized>(_: &UniqueEmail, rng: &mut R) -> String {
        let email: String = SafeEmail().fake_with_rng(rng);
        let id = nanoid!(8, &ALPHABET);
        let at = email.find('@').unwrap();
        format!("{}.{}{}", &email[..at], id, &email[at..])
    }
}
