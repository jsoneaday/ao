use clap::ValueEnum;

#[derive(ValueEnum, Clone, Debug)]
pub enum Method {
    Help = 0,
    Balance = 1,
    Withdraw = 2,
    Upload = 3,
    UploadDir = 4,
    Fund = 5,
    Price = 6,
}
