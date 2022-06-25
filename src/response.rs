use std::{
    borrow::{Borrow, BorrowMut, Cow},
    f64::consts::E,
    ops::Deref,
};

use crate::read::TY;

use super::Reader;
use futures_lite::io;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DataResp<'a> {
    pub Tag: Cow<'a, str>,
    pub Fields: Cow<'a, [TY<'a>]>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Resp<'a> {
    ContReq(ContinuationReq<'a>),
    StatResp(StatusResp<'a>),
    DataResp(DataResp<'a>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatusResp<'a> {
    pub Tag: Cow<'a, str>,
    pub Type: Cow<'a, str>,
    pub Code: Cow<'a, str>,
    pub Arguments: Cow<'a, [TY<'a>]>,
    pub Info: Cow<'a, str>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContinuationReq<'a> {
    pub Info: Cow<'a, str>,
}

impl<'r, T: io::AsyncReadExt + Unpin + Send> Reader<T> {
    pub async fn ReadResp<'b>(&mut self) -> io::Result<Resp<'b>> {
        let tag = self.ReadAtom().await?;

        if tag == "+" {
            if self.ReadSp().await.is_ok() {
                self.UnReadRune().await;
            }

            let mut resp = ContinuationReq::default();

            resp.Info = self.ReadInfo().await?;

            return Ok(Resp::ContReq(resp));
        }

        self.ReadSp().await?;

        // Can be either data or status
        // Try to parse a status
        let mut fields = Cow::<'b, [TY<'b>]>::default();

        if let Ok(atom) = self.ReadAtom().await {
            if self.ReadSp().await.is_ok() {
                // TODO: String parse check
                if ["OK", "NO", "BAD", "PREAUTH", "BYE"].contains(&atom.as_ref()) {
                    let mut resp = StatusResp::default();
                    resp.Tag = tag;
                    resp.Type = atom;

                    let char = self.ReadRune().await?;
                    self.UnReadRune().await;

                    if char == '[' {
                        let (code, fields) = self.ReadRespCode().await?;
                        resp.Code = code;
                        resp.Arguments = fields;
                    }

                    let info = self.ReadInfo().await?;
                    resp.Info = info;

                    return Ok(Resp::StatResp(resp));
                }
            } else {
                self.UnReadRune().await;
            }

            fields.to_mut().push(TY::Str(atom));
        } else {
            self.UnReadRune().await;
        }

        let mut resp = DataResp::default();
        resp.Tag = tag;

        let mut remaining = self.ReadLine().await?;

        fields.to_mut().append(remaining.to_mut());
        resp.Fields = fields;

        Ok(Resp::DataResp(resp))
    }
}

pub async fn ParseNamedResp<'a, 'b>(
    resp: &'b DataResp<'a>,
) -> io::Result<(Cow<'a, str>, Cow<'a, [TY<'a>]>)> {
    let mut f = resp.Fields.clone();

    let mut flip = false;
    let name = match (f.get(0), f.get(1)) {
        (Some(TY::Str(number)), Some(TY::Str(name))) if number.parse::<usize>().is_ok() => {
            flip = true;
            name.to_owned()
        }
        (Some(TY::Str(name)), _) => name.to_owned(),
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "named response contains no fields",
            ));
        }
    };

    if flip {
        f.to_mut().swap(0, 1)
    }

    f.to_mut().drain(..1);

    return Ok((name, f));
}
