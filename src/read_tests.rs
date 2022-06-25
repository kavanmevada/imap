use crate::Reader;
use futures_lite::AsyncReadExt;

#[test]
fn TestReader_ReadSp() {
    smol::block_on(async {
        debug_assert!(Reader::from(b" ".bytes()).ReadSp().await.is_ok());
        debug_assert!(Reader::from(b"".bytes()).ReadSp().await.is_err());
    })
}

#[test]
fn TestReader_ReadCrlf() {
    smol::block_on(async {
        debug_assert!(Reader::from(b"\r\n".bytes()).ReadCrlf().await.is_ok());
        debug_assert!(Reader::from(b"".bytes()).ReadCrlf().await.is_err());
        debug_assert!(Reader::from(b"\n".bytes()).ReadCrlf().await.is_ok());
        debug_assert!(Reader::from(b"\r".bytes()).ReadCrlf().await.is_err());
        debug_assert!(Reader::from(b"\r42".bytes()).ReadCrlf().await.is_err());
    })
}

#[test]
fn TestReader_ReadAtom() {
    smol::block_on(async {
        debug_assert!(Reader::from(b"NIL\r\n".bytes()).ReadAtom().await.is_ok());
        let mut r = Reader::from(b"atom\r\n".bytes());
        debug_assert!(r.ReadAtom().await.map_or(false, |a| a == "atom"));
        debug_assert!(r.ReadCrlf().await.is_ok());
        debug_assert!(r.ReadRune().await.is_err());

        debug_assert!(Reader::from(b"".bytes()).ReadAtom().await.is_err());
        debug_assert!(Reader::from(b"(hi there)\r\n".bytes())
            .ReadAtom()
            .await
            .is_err());
        debug_assert!(Reader::from(b"{42}\r\n".bytes()).ReadAtom().await.is_err());
        debug_assert!(Reader::from(b"\"\r\n".bytes()).ReadAtom().await.is_err());
        debug_assert!(Reader::from(b"abc]".bytes()).ReadAtom().await.is_err());
        debug_assert!(Reader::from(b"[abc]def]ghi".bytes())
            .ReadAtom()
            .await
            .is_err());
    })
}

#[test]
fn TestReader_ReadLiteral() {
    smol::block_on(async {
        let mut r = Reader::from(b"{7}\r\nabcdefg".bytes());
        debug_assert!(r.ReadLiteral().await.map_or(false, |a| a == "abcdefg"));

        debug_assert!(Reader::from(b"".bytes()).ReadLiteral().await.is_err());

        debug_assert!(Reader::from(b"[7}\r\nabcdefg".bytes())
            .ReadLiteral()
            .await
            .is_err());
        debug_assert!(Reader::from(b"{7]\r\nabcdefg".bytes())
            .ReadLiteral()
            .await
            .is_err());
        debug_assert!(Reader::from(b"{7.4}\r\nabcdefg".bytes())
            .ReadLiteral()
            .await
            .is_err());
        debug_assert!(Reader::from(b"{7}abcdefg".bytes())
            .ReadLiteral()
            .await
            .is_err());
        debug_assert!(Reader::from(b"{7}\rabcdefg".bytes())
            .ReadLiteral()
            .await
            .is_err());
        debug_assert!(Reader::from(b"{7}\nabcdefg".bytes())
            .ReadLiteral()
            .await
            .is_ok());
        debug_assert!(Reader::from(b"{7}\r\nabcd".bytes())
            .ReadLiteral()
            .await
            .is_err());
    })
}

#[test]
fn TestReader_ReadQuotedString() {
    smol::block_on(async {
        let mut r = Reader::from(b"\"hello gopher\"\r\n".bytes());
        debug_assert!(r
            .ReadQuotedString()
            .await
            .map_or(false, |a| a == "hello gopher"));
        debug_assert!(r.ReadCrlf().await.is_ok());
        debug_assert!(r.ReadRune().await.is_err());

        debug_assert!(Reader::from(
            b"\"here's a backslash: \\\\, and here's a double quote: \\\" !\"\r\n".bytes()
        )
        .ReadQuotedString()
        .await
        .map_or(false, |a| a
            == "here's a backslash: \\, and here's a double quote: \" !"));

        debug_assert!(Reader::from(b"".bytes()).ReadQuotedString().await.is_err());
        debug_assert!(Reader::from(b"hello gopher\"\r\n".bytes())
            .ReadQuotedString()
            .await
            .is_err());
        debug_assert!(Reader::from(b"\"hello gopher\r\n".bytes())
            .ReadQuotedString()
            .await
            .is_err());
        debug_assert!(Reader::from(b"\"hello \\gopher\"\r\n".bytes())
            .ReadQuotedString()
            .await
            .is_err());
    })
}

#[test]
fn TestReader_ReadFields() {
    smol::block_on(async {
        let mut r = Reader::from(b"field1 \"field2\"\r\n".bytes());
        debug_assert!(r
            .ReadFields()
            .await
            .map_or(false, |a| a[0] == "field1" && a[1] == "field2"));
        debug_assert!(r.ReadCrlf().await.is_ok());
        debug_assert!(r.ReadRune().await.is_err());

        debug_assert!(Reader::from(b"".bytes()).ReadFields().await.is_err());
        debug_assert!(Reader::from(b"fi\"eld1 \"field2\"\r\n".bytes())
            .ReadFields()
            .await
            .is_err());
        debug_assert!(Reader::from(b"field1 ".bytes()).ReadFields().await.is_err());

        debug_assert!(Reader::from(b"field1 (".bytes())
            .ReadFields()
            .await
            .is_err());

        debug_assert!(Reader::from(b"field1\"field2\"\r\n".bytes())
            .ReadFields()
            .await
            .is_err());

        debug_assert!(Reader::from(b"\"field1\"\"field2\"\r\n".bytes())
            .ReadFields()
            .await
            .is_err());
    })
}

#[test]
fn TestReader_ReadFields2() {
    smol::block_on(async {
        let mut r = Reader::from(b"(field1 \"field2\") field1\r\n".bytes());
        dbg!(r.ReadFields2().await);
    })
}

#[test]
fn TestReader_ReadList() {
    smol::block_on(async {
        let mut r = Reader::from(b"(field1 \"field2\" {6}\r\nfield3 field4)".bytes());
        debug_assert!(r.ReadList().await.map_or(false, |a| a[0] == "field1"
            && a[1] == "field2"
            && a[2] == "field3"
            && a[3] == "field4"));
        debug_assert!(r.ReadRune().await.is_err());

        debug_assert!(Reader::from(b"()".bytes())
            .ReadList()
            .await
            .map_or(false, |a| a.is_empty()));

        debug_assert!(Reader::from(b"".bytes()).ReadList().await.is_err());
        debug_assert!(Reader::from(b"[field1 field2 field3)".bytes())
            .ReadList()
            .await
            .is_err());
        debug_assert!(Reader::from(b"(field1 fie\"ld2 field3)".bytes())
            .ReadList()
            .await
            .is_err());
        debug_assert!(Reader::from(b"(field1 field2 field3\r\n".bytes())
            .ReadList()
            .await
            .is_err());
    })
}

// #[test]
// fn TestReader_ReadLine() {
//     smol::block_on(async {
//         let mut r = Reader::from(b"field1 field2\r\n".bytes());
//         debug_assert!(r
//             .ReadLine()
//             .await
//             .map_or(false, |a| a[0] == "field1" && a[1] == "field2"));
//         debug_assert!(r.ReadRune().await.is_err());

//         debug_assert!(Reader::from(b"".bytes()).ReadList().await.is_err());
//         debug_assert!(Reader::from(b"field1 field2\rabc".bytes())
//             .ReadList()
//             .await
//             .is_err());
//     })
// }

// #[test]
// fn TestReader_ReadRespCode() {
//     smol::block_on(async {
//         let mut r = Reader::from(b"[CAPABILITY NOOP STARTTLS]".bytes());
//         debug_assert!(r
//             .ReadRespCode()
//             .await
//             .map_or(false, |(code, fields)| code == "CAPABILITY"
//                 && fields[0] == "NOOP"
//                 && fields[1] == "STARTTLS"));
//         debug_assert!(r.ReadRune().await.is_err());

//         debug_assert!(Reader::from(b"".bytes()).ReadList().await.is_err());
//         debug_assert!(Reader::from(b"{CAPABILITY NOOP STARTTLS]".bytes())
//             .ReadList()
//             .await
//             .is_err());
//         debug_assert!(Reader::from(b"[CAPABILITY NO\"OP STARTTLS]".bytes())
//             .ReadList()
//             .await
//             .is_err());
//         debug_assert!(Reader::from(b"[]".bytes()).ReadList().await.is_err());
//         debug_assert!(Reader::from(b"[{3}\r\nabc]".bytes())
//             .ReadList()
//             .await
//             .is_err());
//         debug_assert!(Reader::from(b"[CAPABILITY NOOP STARTTLS\r\n".bytes())
//             .ReadList()
//             .await
//             .is_err());
//     })
// }

#[test]
fn TestReader_ReadInfo() {
    smol::block_on(async {
        let mut r = Reader::from(b"I love potatoes.\r\n".bytes());
        debug_assert!(r
            .ReadInfo()
            .await
            .map_or(false, |str| str == "I love potatoes."));
        debug_assert!(r.ReadRune().await.is_err());

        debug_assert!(Reader::from(b"I love potatoes.".bytes())
            .ReadInfo()
            .await
            .is_err());

        debug_assert!(Reader::from(b"I love potatoes.\r".bytes())
            .ReadInfo()
            .await
            .is_err());

        debug_assert!(Reader::from(b"I love potatoes.\n".bytes())
            .ReadInfo()
            .await
            .is_ok());

        debug_assert!(Reader::from(b"I love potatoes.\rabc".bytes())
            .ReadInfo()
            .await
            .is_err());
    })
}
