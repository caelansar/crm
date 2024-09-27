use std::sync::Arc;

use dotenv::dotenv;
use load_balancer::{DecodingKey, PK};
use pingora::protocols::ALPN;
use pingora::server::{configuration::Opt, Server};
use pingora::tls::x509::X509;
use pingora::Result;
use pingora::{
    proxy::{ProxyHttp, Session},
    upstreams::peer::HttpPeer,
};
use pingora_load_balancing::selection::RoundRobin;
use pingora_load_balancing::LoadBalancer;
use tonic::async_trait;
use tracing::{info, Level};

pub struct GrpcProxy(Arc<GrpcProxyInner>);

pub struct GrpcProxyInner {
    lb: LoadBalancer<RoundRobin>,
    dk: DecodingKey,
}

fn main() {
    dotenv().ok();

    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let opt = Opt::default();
    let mut server = Server::new(Some(opt)).unwrap();
    server.bootstrap();

    let dk = DecodingKey::load(PK).unwrap();

    let upstreams: LoadBalancer<RoundRobin> =
        LoadBalancer::try_from_iter(["127.0.0.1:50002", "127.0.0.1:50003"]).unwrap();

    let mut grpc_proxy = pingora::proxy::http_proxy_service(
        &server.configuration,
        GrpcProxy(Arc::new(GrpcProxyInner { lb: upstreams, dk })),
    );

    let mut tls_settings = pingora::listeners::TlsSettings::intermediate(
        "assets/cert/server.crt",
        "assets/cert/server.key",
    )
    .unwrap();

    // set alpn to h2 so that the client can use http2
    tls_settings.enable_h2();

    grpc_proxy.add_tls_with_settings("0.0.0.0:8000", None, tls_settings);

    server.add_service(grpc_proxy);

    server.run_forever();
}

#[async_trait]
impl ProxyHttp for GrpcProxy {
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {}

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool> {
        info!("request_filter");

        let token = session.req_header().headers.get("authorization");

        if let Some(token) = token {
            match self.0.dk.verify(token.to_str().unwrap()) {
                Ok(user) => {
                    info!("Authenticated user: {:?}", user);
                    Ok(false) // Continue processing the request
                }
                Err(e) => {
                    info!("Authentication error: {:?}", e);
                    let _ = session.respond_error(401).await;
                    Ok(true) // Stop processing the request
                }
            }
        } else {
            info!("Missing authorization token");
            let _ = session.respond_error(401).await;
            Ok(true) // Stop processing the request
        }
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        // get the upstream peer using round robin
        let upstream = self
            .0
            .lb
            .select(b"", 256) // key is ignored if the selection is random or round robin.
            .unwrap();

        info!("upstream peer is: {:?}", upstream);

        let mut peer = Box::new(HttpPeer::new(upstream, false, String::default()));
        // set alpn to h2 so that the client can use http2
        peer.options.alpn = ALPN::H2;
        // trust the ca in assets/cert/ca.crt
        peer.options.ca = Some(Arc::new(Box::from(vec![X509::from_pem(include_bytes!(
            "../assets/cert/ca.crt"
        ))
        .unwrap()])));
        Ok(peer)
    }
}
