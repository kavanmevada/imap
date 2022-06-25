use std::borrow::Cow;

use super::Handler;
use crate::{
    read::TY,
    response::{self, Resp},
};
use async_trait::async_trait;
use futures_lite::io;

#[derive(Debug, Default, Clone)]
pub struct List<'s> {
    pub Mailboxes: Cow<'s, [MailboxInfo<'s>]>,
}

#[derive(Debug, Default, Clone)]
pub struct MailboxInfo<'m> {
    Attributes: Cow<'m, [Cow<'m, str>]>,
    Delimiter: Cow<'m, str>,
    Name: Cow<'m, str>,
}

impl<'m> MailboxInfo<'m> {
    fn Parse(fields: Cow<'_, [TY<'m>]>) -> io::Result<MailboxInfo<'m>> {
        let mut mbox = MailboxInfo::default();

        if fields.len() < 3 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Mailbox info needs at least 3 fields",
            ));
        }

        if let Some(list) = fields.get(0).map_or(None, |a| {
            if let TY::List(list) = a {
                Some(list)
            } else {
                None
            }
        }) {
            mbox.Attributes = list.to_owned();
        }

        mbox.Delimiter = match fields.get(1) {
            Some(TY::Str(name)) => name.to_owned(),
            _ => " ".into(),
        };

        mbox.Name = match fields.get(2) {
            Some(TY::Str(name)) => name.to_owned(),
            _ => " ".into(),
        };

        Ok(mbox)
    }
}

#[async_trait]
impl<'s> Handler<'s> for List<'s> {
    async fn Handle(&mut self, resp: &mut Resp<'s>) -> io::Result<()> {
        if let Resp::DataResp(resp) = resp {
            let (name, fields) = response::ParseNamedResp(resp).await?;
            if name == "LIST" {
                self.Mailboxes.to_mut().push(MailboxInfo::Parse(fields)?);
            }
        }

        Ok(())
    }
}
