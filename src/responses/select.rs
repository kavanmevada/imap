use std::borrow::Cow;

use super::Handler;
use crate::{
    read::TY,
    response::{self, Resp, StatusResp},
};
use async_trait::async_trait;
use futures_lite::{io, FutureExt};

#[derive(Debug, Default, Clone)]
pub struct Select<'s> {
    pub Mailbox: MailboxStatus<'s>,
}

#[derive(Debug, Default, Clone)]
pub struct MailboxStatus<'m> {
    pub Name: Cow<'m, str>,
    pub ReadOnly: bool,
    pub Items: Cow<'m, [Cow<'m, str>]>,
    pub Flags: Cow<'m, [Cow<'m, str>]>,
    pub UnseenSeqNum: usize,
    pub PermanentFlags: Cow<'m, [Cow<'m, str>]>,
    pub UidNext: usize,
    pub UidValidity: usize,
    pub Messages: usize,
    pub Recents: usize,
}

#[async_trait]
impl<'s> Handler<'s> for Select<'s> {
    async fn Handle(&mut self, resp: &mut Resp<'s>) -> io::Result<()> {
        match resp {
            Resp::ContReq(resp) => todo!(),
            Resp::StatResp(StatusResp {
                Code, Arguments, ..
            }) => match Code.as_ref() {
                "UNSEEN" => {
                    self.Mailbox.UnseenSeqNum = match Arguments.get(0) {
                        Some(TY::Str(name)) => name.parse::<usize>().unwrap_or_default(),
                        _ => 0,
                    };
                }
                "PERMANENTFLAGS" => {
                    if let Some(list) = Arguments.get(0).map_or(None, |a| {
                        if let TY::List(list) = a {
                            Some(list)
                        } else {
                            None
                        }
                    }) {
                        self.Mailbox.PermanentFlags = list.to_owned();
                    }
                }
                "UIDNEXT" => {
                    self.Mailbox.UidNext = match Arguments.get(0) {
                        Some(TY::Str(name)) => name.parse::<usize>().unwrap_or_default(),
                        _ => 0,
                    };
                }
                "UIDVALIDITY" => {
                    self.Mailbox.UidValidity = match Arguments.get(0) {
                        Some(TY::Str(name)) => name.parse::<usize>().unwrap_or_default(),
                        _ => 0,
                    };
                }
                a => {
                    dbg!(a);
                    todo!()
                }
            },
            Resp::DataResp(resp) => {
                let (name, fields) = response::ParseNamedResp(&resp).await?;
                match name.as_ref() {
                    "FLAGS" => {
                        if let Some(list) = fields.get(0).map_or(None, |a| {
                            if let TY::List(list) = a {
                                Some(list)
                            } else {
                                None
                            }
                        }) {
                            self.Mailbox.Flags = list.to_owned();
                        }
                    }
                    "EXISTS" => {
                        self.Mailbox.Messages = match fields.get(0) {
                            Some(TY::Str(name)) => name.parse::<usize>().unwrap_or_default(),
                            _ => 0,
                        };
                    }
                    "RECENT" => {
                        self.Mailbox.Recents = match fields.get(0) {
                            Some(TY::Str(name)) => name.parse::<usize>().unwrap_or_default(),
                            _ => 0,
                        };
                    }
                    _ => todo!(),
                }
            }
        };

        Ok(())
    }
}
