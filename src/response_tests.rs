use std::borrow::Cow;

use crate::{
    read::TY,
    response::{DataResp, Resp},
    Reader,
};
use futures_lite::AsyncReadExt;

#[test]
fn TestReadResp_ContinuationReq() {
    smol::block_on(async {
        debug_assert!(Reader::from(b"+ send literal\r\n".bytes())
            .ReadResp()
            .await
            .map_or(false, |resp| if let Resp::ContReq(cont) = resp {
                cont.Info == "send literal"
            } else {
                false
            }));
    })
}

#[test]
fn TestReadResp_ContinuationReq_NoInfo() {
    smol::block_on(async {
        debug_assert!(Reader::from(b"+\r\n".bytes())
            .ReadResp()
            .await
            .map_or(false, |resp| if let Resp::ContReq(cont) = resp {
                cont.Info == ""
            } else {
                false
            }));
    })
}

#[test]
fn TestReadResp_Resp() {
    smol::block_on(async {
        debug_assert!(Reader::from(b"* 1 EXISTS\r\n".bytes())
            .ReadResp()
            .await
            .map_or(false, |resp| if let Resp::DataResp(data) = resp {
                data.Tag == "*" && data.Fields.len() == 2
            } else {
                false
            }));
    })
}

#[test]
fn TestReadResp_Resp_NoArgs() {
    smol::block_on(async {
        debug_assert!(Reader::from(b"* SEARCH\r\n".bytes())
            .ReadResp()
            .await
            .map_or(false, |resp| if let Resp::DataResp(data) = resp {
                data.Tag == "*"
                    && data.Fields.len() == 1
                    && data.Fields[0] == TY::Str("SEARCH".into())
            } else {
                false
            }));
    })
}

#[test]
fn TestReadResp_StatusResp() {
    smol::block_on(async {
        debug_assert!(Reader::from(b"* OK IMAP4rev1 Service Ready\r\n".bytes())
            .ReadResp()
            .await
            .map_or(false, |resp| if let Resp::StatResp(status) = resp {
                status.Tag == "*" && status.Type == "OK" && status.Info == "IMAP4rev1 Service Ready"
            } else {
                false
            }));

        debug_assert!(Reader::from(b"* PREAUTH Welcome Pauline!\r\n".bytes())
            .ReadResp()
            .await
            .map_or(false, |resp| if let Resp::StatResp(status) = resp {
                status.Tag == "*" && status.Type == "PREAUTH" && status.Info == "Welcome Pauline!"
            } else {
                false
            }));

        debug_assert!(Reader::from(b"a001 OK NOOP completed\r\n".bytes())
            .ReadResp()
            .await
            .map_or(false, |resp| if let Resp::StatResp(status) = resp {
                status.Tag == "a001" && status.Type == "OK" && status.Info == "NOOP completed"
            } else {
                false
            }));

        debug_assert!(
            Reader::from(b"a001 OK [READ-ONLY] SELECT completed\r\n".bytes())
                .ReadResp()
                .await
                .map_or(false, |resp| if let Resp::StatResp(status) = resp {
                    status.Tag == "a001"
                        && status.Type == "OK"
                        && status.Code == "READ-ONLY"
                        && status.Info == "SELECT completed"
                } else {
                    false
                })
        );

        debug_assert!(Reader::from(
            b"a001 OK [CAPABILITY IMAP4rev1 UIDPLUS] LOGIN completed\r\n".bytes()
        )
        .ReadResp()
        .await
        .map_or(false, |resp| if let Resp::StatResp(status) = resp {
            status.Tag == "a001"
                && status.Type == "OK"
                && status.Code == "CAPABILITY"
                && status.Arguments.as_ref()
                    == [TY::Str("IMAP4rev1".into()), TY::Str("UIDPLUS".into())]
                && status.Info == "LOGIN completed"
        } else {
            false
        }));
    })
}

#[test]
fn TestParseNamedResp() {
    smol::block_on(async {
        let mut fields = Cow::<'_, [TY<'_>]>::default();
        fields.to_mut().push(TY::Str("CAPABILITY".into()));
        fields.to_mut().push(TY::Str("IMAP4rev1".into()));

        let mut resp = DataResp {
            Tag: "*".into(),
            Fields: fields,
        };

        super::response::ParseNamedResp(&mut resp)
            .await
            .map_or(false, |(name, fields)| {
                name == "CAPABILITY" && fields[0] == TY::Str("IMAP4rev1".into())
            });
    })
}
