use crate::tags::Tag;

pub struct Signer {
    pub signer: Vec<u8>,
    pub public_key: Vec<u8>,
    pub signature_type: i64,
    pub signature_length: i64,
    pub owner_length: i64,
    pub pem: String
}

pub struct Options;

// pub trait SignerMaker {
//     async fn sign(message: Vec<u8>, _opts: Option<Options>): Result<Vec<u8>, ArBundleErrors>>;
//     async fn sign_data_item?(data_item: string | Buffer, tags: Vec<Tag>): Promise<DataItem>;
//     async fn set_public_key?(): Promise<void>;
//     async fn get_address?(): Promise<string>;
//     fn verify(_pk: string | Buffer, _message: Uint8Array, _signature: Uint8Array, _opts?: any): boolean {
//         throw new Error('You must implement verify method on child');
//     }
// }