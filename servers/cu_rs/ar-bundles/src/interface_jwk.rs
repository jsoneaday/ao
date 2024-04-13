#[allow(unused)]
pub struct JWKPublicInterface {
    pub kty: String,
    pub e: String,
    pub n: String
}

#[allow(unused)]
pub struct JWKInterface {
    pub base: JWKPublicInterface,
    d: Option<String>,
    p: Option<String>,
    q: Option<String>,
    dp: Option<String>,
    dq: Option<String>,
    di: Option<String>,
}