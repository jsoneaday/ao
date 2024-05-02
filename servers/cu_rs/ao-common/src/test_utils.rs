static WALLET_FILE: once_cell::sync::OnceCell<String> = once_cell::sync::OnceCell::new();
pub fn get_wallet_file() -> &'static String {
    WALLET_FILE.get_or_init(|| {
        dotenv::dotenv().ok();

        std::env::var("WALLET_FILE").unwrap()
    })
}

static UPLOADER_URL: once_cell::sync::OnceCell<String> = once_cell::sync::OnceCell::new();
pub fn get_uploader_url() -> &'static String {
    UPLOADER_URL.get_or_init(|| {
        dotenv::dotenv().ok();

        std::env::var("UPLOADER_URL").unwrap()
    })
}

static GATEWAY_URL: once_cell::sync::OnceCell<String> = once_cell::sync::OnceCell::new();
pub fn get_gateway_url() -> &'static String {
    GATEWAY_URL.get_or_init(|| {
        dotenv::dotenv().ok();

        std::env::var("GATEWAY_URL").unwrap()
    })
}

static GRAPHQL_URL: once_cell::sync::OnceCell<String> = once_cell::sync::OnceCell::new();
pub fn get_graphql_url() -> &'static String {
    GRAPHQL_URL.get_or_init(|| {
        dotenv::dotenv().ok();

        std::env::var("GRAPHQL_URL").unwrap()
    })
}

static ARWEAVE_URL: once_cell::sync::OnceCell<String> = once_cell::sync::OnceCell::new();
pub fn get_arweave_url() -> &'static String {
    ARWEAVE_URL.get_or_init(|| {
        dotenv::dotenv().ok();

        std::env::var("ARWEAVE_URL").unwrap()
    })
}