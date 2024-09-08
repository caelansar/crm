#![feature(impl_trait_in_assoc_type)]

use std::vec;

use anyhow::Result;
use crm::pb::{
    user_service_server::{UserService, UserServiceServer},
    CreateUserRequest, GetUserRequest, User,
};
use tonic::{async_trait, transport::Server, Request, Response, Status};

#[derive(Default)]
pub struct UserServer {}

#[async_trait]
impl UserService for UserServer {
    type ServerSideStreamingStream =
        impl tokio_stream::Stream<Item = Result<User, Status>> + Send + 'static;

    async fn get_user(&self, request: Request<GetUserRequest>) -> Result<Response<User>, Status> {
        let input = request.into_inner();
        println!("get_user: {:?}", input);
        Ok(Response::new(User::default()))
    }

    async fn server_side_streaming(
        &self,
        _request: Request<GetUserRequest>,
    ) -> Result<Response<Self::ServerSideStreamingStream>, Status> {
        let user1 = User::new(1, "John Doe", "john.doe@example.com");
        let user2 = User::new(2, "Cae", "cae@example.com");
        let stream = tokio_stream::iter(vec![Ok(user1), Ok(user2)]);
        Ok(Response::new(stream))
    }

    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<User>, Status> {
        let input = request.into_inner();
        println!("create_user: {:?}", input);
        let user = User::new(1, &input.name, &input.email);
        Ok(Response::new(user))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:50051".parse().unwrap();
    let svc = UserServer::default();

    println!("UserService listening on {}", addr);

    Server::builder()
        .add_service(UserServiceServer::new(svc))
        .serve(addr)
        .await?;
    Ok(())
}
