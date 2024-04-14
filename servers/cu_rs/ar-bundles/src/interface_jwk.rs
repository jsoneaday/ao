use serde::Deserialize;

#[allow(unused)]
#[derive(Deserialize)]
pub struct JWKInterface {
    pub kty: String,
    pub e: String,
    pub n: String,
    d: Option<String>,
    p: Option<String>,
    q: Option<String>,
    dp: Option<String>,
    dq: Option<String>,
    di: Option<String>,
}