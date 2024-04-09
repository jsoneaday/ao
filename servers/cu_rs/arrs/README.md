ARRS
====

Introduction
------------

ARRS is a Rust API implementation of the Arweave client. It can be
used to write command line, desktop, or web programs in Rust to use
most features of Arweave, including creating, importing, and exporting
wallets, checking balance, sending transactions, uploading files,
etc..

Arweave is a permanent storage network based on crypto. For more
information, please visit: <https://arweave.org/>

Usage
-----

This program needs OpenSSL. It will download the newest official
release of the OpenSSL, and automatically compiles it. Therefore your
system needs to have a C compiler, Perl (and perl-core), and make
installed.

This program has async functions, and it can be used for developments
of web applications, but to test this program locally, you need to add
`tokio` to your dependencies.

Although most modules and structs are made in public and they can be
used individually, this program intends to wrap everything into the
struct of ArWallet. You should be able to find most Arweave
functionalities and features from the methods of ArWallet struct.

You can find the documentation here: <https://docs.rs/arrs>, or you
can compile and open the documentation locally by running: `cargo doc
--no-deps --open`

Example
-------

This program has async functions, so to test this program locally, you
need to add `tokio` and `arrs` into your Cargo.toml:

```
[dependencies]
arrs = {path = "../arrs"}
tokio = { version = "1.35.1", features = ["full"] }
```

### Print Wallet Address and AR Balance:

```
use arrs::wallet::ArWallet;
use tokio;

#[tokio::main]
async fn main() {
    let arwallet = ArWallet::new();
    let balance = arwallet.balance();
    let address = arwallet.address();
    println!("Your Arweave Wallet Address is: ");
    println!("{}", address);
    println!("Your AR balance is {}: ", balance.await.unwrap());
}
```

### Import a JSON Web Key (JWK)

```
use arrs::wallet::ArWallet;
use tokio;
use std::fs::read_to_string;

#[tokio::main]
async fn main() {
    let jwk = read_to_string("./test.json").unwrap();
    let arwallet = ArWallet::from_jwk(&jwk);
    let balance = arwallet.balance();
    let address = arwallet.address();
    println!("Your Arweave Wallet Address is: ");
    println!("{}", address);
    println!("Your AR balance is {}: ", balance.await.unwrap());
}
```

### Create an AR transfer transaction, sign, and submit.

```
use arrs::wallet::ArWallet;
use tokio;
use std::fs::read_to_string;

#[tokio::main]
async fn main() {
    let jwk = read_to_string("./test.json").unwrap();
    let ar = ArWallet::from_jwk(&jwk);
    let mut tx = ar.create_ar_transaction(
        "lKoYAKxF_ESjG500WfCfJvcxM83OFHGP0tnkIMnUJfM",
        0.02
    ).await;
    tx.add_tag("Test Name 2", "Test Value 2");
    tx.sign();
    println!("{:?}", tx.submit().await);
}
```

### Create a data transaction, add tags, sign, submit, and upload

```
use arrs::wallet::ArWallet;
use tokio;
use std::{
    fs::{File, read_to_string},
    io::Read
};

#[tokio::main]
async fn main() {
    let jwk = read_to_string("./test.json").unwrap();
    let mut ar = ArWallet::from_jwk(&jwk);
    let mut data_file = File::open("./the-freedom-of-constraint.mp4").unwrap();
    let mut raw_data = Vec::new();
    data_file.read_to_end(&mut raw_data).unwrap();   
    let mut tx = ar.create_data_transaction(&raw_data).await;
    tx.add_tag("Content-Type", "video/mp4");
    tx.add_tag("Test Name 2", "Test Value 2");
    tx.sign();
    println!("{:?}", tx.submit().await);
    while ar.uploader().current_idx() < ar.uploader().chunk_size() {
        println!("{}", ar.upload().await.unwrap());
    }
}
```

### Download a file

Many Arweave features, such as downloading a file, do not need a
arweave wallet (key) to operate. Therefore, in this example, we import
ArPublic instead of ArWallet. This is useful for many cases, such as
building a website for search and downloading files without
registration.

```
use arrs::public_client::ArPublic;
use tokio;
use std::fs::{write as write_file};

#[tokio::main]
async fn main() {
    let mut ap = ArPublic::new();
    ap.new_download("jVid2TTQ1g9j_sU6_4AlJm6hkAKiNznUYLCTEx-fobk")
        .await.unwrap();
    while ap.downloader().current_offset()
        <= ap.downloader().end_point()
    {
        let finished_percent= ap.download().await.unwrap();
        println!("Downloading ... {}%", finished_percent);
    }
    // Assume that you know the file's original content type. If you
    // don't, you can try to check if there is a Content-Type tag in
    // the transaction.
    let downloaded_data = &ap.downloader().downloaded_data();
    write_file("./test.mp4", downloaded_data).unwrap();
}

```


Copyright
---------

Copyright © 2024 IceGuye

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Lesser General Public License as
published by the Free Software Foundation, version 3 of the License.

This program is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
Lesser General Public License for more details.

You should have received a copy of the GNU Lesser General Public
License along with this program.  If not, see
<http://www.gnu.org/licenses/>