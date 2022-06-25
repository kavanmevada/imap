# imap-rs
Library is currently in development.

Documentation:
- [latest release](https://docs.rs/imap-rs)

Usage:
```
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
```

## Contributing

Contributions are always welcome! If you have an idea, it's best to float it by me before working on
it to ensure no effort is wasted. If there's already an open issue for it, knock yourself out.

[Discussions]: https://github.com/kavanmevada/imap-rs/discussions

## License

This project is licensed under either of

- [Apache License, Version 2.0](https://github.com/kavanmevada/imap-rs/blob/main/LICENSE)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
time by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.