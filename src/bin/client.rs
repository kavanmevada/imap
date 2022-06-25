use imap_rs::Client;
use std::{env, io};

fn main() -> io::Result<()> {
    smol::block_on(async {
        let mut c = Client::DialTLS(env!("HOST"), format!("{}:{}", env!("HOST"), 993)).await?;
        c.handleGreetAndStartReading().await?;

        let _ = c.Login(env!("EMAIL"), env!("PASS")).await?;
        let _ = c.List("\"\"", "*").await?;
        let selected = c.Select("Inbox", false).await?;

        println!("{:#?}", selected);

        Ok::<(), std::io::Error>(())
    })
}
