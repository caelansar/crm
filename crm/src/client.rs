use anyhow::Result;
use crm::pb::{crm_client::CrmClient, WelcomeRequestBuilder};
use tonic::{transport::Channel, Request};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let channel = Channel::from_static("http://127.0.0.1:50000")
        .connect()
        .await?;

    let mut client = CrmClient::new(channel);

    let req = WelcomeRequestBuilder::default()
        .id(Uuid::new_v4().to_string())
        .interval(90u32)
        .content_ids(vec![1, 2, 3])
        .build()?;

    let response = client.welcome(Request::new(req)).await?.into_inner();
    println!("Response: {:?}", response);
    Ok(())
}
