use jwt_simple::prelude::*;
use tonic::{service::Interceptor, Request, Status};
use tracing::info;

pub const SK: &str = "-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIDnxJGEJGoW+mNKHn4vRY1V6BQ3MglSQSuZ8featmyC4
-----END PRIVATE KEY-----";
pub const PK: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAfM+lwNHj6TRJ3EGP38lIJcOo9Dlt2u2JzcwWMbu7jQY=
-----END PUBLIC KEY-----";

#[derive(Debug, Clone)]
pub struct DecodingKey(Ed25519PublicKey);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub name: String,
    pub email: String,
}

const JWT_DURATION: u64 = 60 * 60 * 24 * 7;
const JWT_ISS: &str = "crm_server";
const JWT_AUD: &str = "crm";

pub struct EncodingKey(Ed25519KeyPair);

impl EncodingKey {
    pub fn load(pem: &str) -> Result<Self, jwt_simple::Error> {
        Ok(Self(Ed25519KeyPair::from_pem(pem)?))
    }

    pub fn sign(&self, user: impl Into<User>) -> Result<String, jwt_simple::Error> {
        let claims = Claims::with_custom_claims(user.into(), Duration::from_secs(JWT_DURATION));
        let claims = claims.with_issuer(JWT_ISS).with_audience(JWT_AUD);
        self.0.sign(claims)
    }
}

impl DecodingKey {
    pub fn load(pem: &str) -> Result<Self, jwt_simple::Error> {
        Ok(Self(Ed25519PublicKey::from_pem(pem)?))
    }

    #[allow(unused)]
    pub fn verify(&self, token: &str) -> Result<User, jwt_simple::Error> {
        let opts = VerificationOptions {
            allowed_issuers: Some(HashSet::from_strings(&[JWT_ISS])),
            allowed_audiences: Some(HashSet::from_strings(&[JWT_AUD])),
            ..Default::default()
        };

        let claims = self.0.verify_token::<User>(token, Some(opts))?;
        Ok(claims.custom)
    }
}

impl Interceptor for DecodingKey {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        let token = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok());
        info!("token: {:?}", token);
        let user = match token {
            Some(bearer) => {
                let token = bearer
                    .strip_prefix("Bearer ")
                    .ok_or_else(|| Status::unauthenticated("invalid token format"))?;
                self.verify(token)
                    .map_err(|e| Status::unauthenticated(e.to_string()))?
            }
            None => return Err(Status::unauthenticated("missing token")),
        };

        req.extensions_mut().insert(user);
        Ok(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let ek = EncodingKey::load(SK).unwrap();
        let user = User {
            name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
        };
        let token = ek.sign(user).unwrap();
        let dk = DecodingKey::load(PK).unwrap();
        let user = dk.verify(&token).unwrap();
        assert_eq!(user, user);
    }
}
