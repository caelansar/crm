use anyhow::Result;
use crm::pb::{user_service_client::UserServiceClient, CreateUserRequest, GetUserRequest};
use tokio_stream::StreamExt as _;
use tonic::Request;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = UserServiceClient::connect("http://127.0.0.1:50051").await?;

    let request = Request::new(CreateUserRequest {
        name: "Alice".to_string(),
        email: "alice@acme.org".to_string(),
    });

    let response = client.create_user(request).await?;

    println!("RESPONSE={:?}", response);

    let request = Request::new(GetUserRequest { id: 1 });

    let mut response = client.server_side_streaming(request).await?;

    while let Some(Ok(user)) = response.get_mut().next().await {
        println!("USER={:?}", user);
    }

    Ok(())
}
