use std::{
    borrow::{BorrowMut, Cow},
    io,
    net::SocketAddr,
};

use async_native_tls::{Host, TlsStream};
use futures_lite::{
    io::{ReadHalf, WriteHalf},
    AsyncRead, AsyncWrite, AsyncWriteExt,
};
use smol::net::{AsyncToSocketAddrs, TcpStream};

use crate::{
    read::TY,
    response::{self, DataResp},
    responses::{Login, Select},
};

use super::{
    commands::{self, Commander},
    response::{Resp, StatusResp},
    responses,
    responses::Handler,
    Reader,
};

#[derive(Debug)]
pub struct ConnInfo {
    LocalAddr: SocketAddr,
    PeerAddr: SocketAddr,
}

#[derive(Debug)]
pub struct Client<'a, T: AsyncRead + AsyncWrite + Unpin> {
    pub Inner: ConnInfo,
    pub Reader: Reader<ReadHalf<T>>,
    pub Writer: WriteHalf<T>,
    pub State: ConnState,
    pub capabilities: Cow<'a, [TY<'a>]>,
}

impl<'a> Client<'a, TlsStream<TcpStream>> {
    pub async fn DialTLS<A: AsyncToSocketAddrs, H: Into<Host>>(
        host: H,
        addr: A,
    ) -> io::Result<Client<'a, TlsStream<TcpStream>>> {
        let mut stream =
            async_native_tls::connect(host, smol::net::TcpStream::connect(addr).await?)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        stream.get_mut().set_nodelay(true)?;

        let local_adder = stream.get_ref().local_addr()?;
        let peer_addr = stream.get_ref().peer_addr()?;

        let (r, w) = smol::io::split(stream);

        Ok(Client::<'a, TlsStream<TcpStream>> {
            Inner: ConnInfo {
                LocalAddr: local_adder,
                PeerAddr: peer_addr,
            },
            Reader: Reader::from(r),
            Writer: w,
            State: ConnState::LogoutState,
            capabilities: Default::default(),
        })
    }

    pub async fn handleGreetAndStartReading<'b>(&'b mut self) -> io::Result<()> {
        if let Resp::StatResp(StatusResp {
            Type: r#type,
            Code: code,
            Arguments: args,
            ..
        }) = self.Reader.ReadResp().await?
        {
            match r#type.as_ref() {
                "PREAUTH" => self.State = ConnState::AuthenticatedState,
                "BYE" => self.State = ConnState::LogoutState,
                "OK" => self.State = ConnState::NotAuthenticatedState,
                _ => {
                    self.State = ConnState::LogoutState;
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("invalid greeting received from server: {}", r#type),
                    ));
                }
            }

            if code == "CAPABILITY" {
                self.capabilities.to_mut().append(&mut args.into_owned());
            }
        }

        Ok(())
    }

    pub async fn Login<'b>(&'b mut self, name: &str, pass: &str) -> io::Result<Resp<'a>> {
        Ok(self
            .execute(
                commands::Login {
                    UserName: name,
                    Password: pass,
                },
                responses::Login::default(),
            )
            .await?
            .1)
    }

    pub async fn Select(
        &mut self,
        name: &'a str,
        readOnly: bool,
    ) -> io::Result<(responses::Select<'a>, Resp<'a>)> {
        let mut select = responses::Select::default();
        select.Mailbox.Name = Cow::Borrowed(name);
        select.Mailbox.ReadOnly = readOnly;

        let selected = self
            .execute(
                commands::Select {
                    Mailbox: name,
                    ReadOnly: readOnly,
                },
                select,
            )
            .await?;

        Ok(selected)
    }

    pub async fn List(
        &mut self,
        reference: &'a str,
        name: &'a str,
    ) -> io::Result<(responses::List<'a>, Resp<'a>)> {
        Ok(self
            .execute(
                commands::List {
                    Mailbox: name,
                    Reference: reference,
                    Subscribed: false,
                },
                responses::List::default(),
            )
            .await?)
    }

    pub async fn execute<'e, C, H>(&mut self, cmdr: C, mut h: H) -> io::Result<(H, Resp<'e>)>
    where
        H: Handler<'e>,
        C: Commander,
    {
        let cmd = cmdr.Command();
        self.Writer.write(cmd.as_str().as_bytes()).await?;

        let mut r = Resp::StatResp(Default::default());
        while let Ok(ref mut resp) = self.Reader.ReadResp().await {
            if let Resp::StatResp(StatusResp { Tag: tag, .. }) = resp {
                if tag == &cmd.Tag {
                    r = resp.to_owned();
                    break;
                }
            }

            h.Handle(resp).await?;
        }

        Ok((h, r))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConnState {
    ConnectingState = 0,
    NotAuthenticatedState = 1 << 0,
    AuthenticatedState = 1 << 1,
    SelectedState = Self::AuthenticatedState as isize + 1 << 2,
    LogoutState = 1 << 3,
    ConnectedState = Self::NotAuthenticatedState as isize
        | Self::AuthenticatedState as isize
        | Self::SelectedState as isize,
}
