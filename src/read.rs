use smol::{
    io::{self, AsyncReadExt},
    Async,
};
use std::{
    borrow::{BorrowMut, Cow},
    future::Future,
    io::Bytes,
    pin::Pin,
    process::Output,
};

use super::{
    cr, dquote, lf, listEnd, listStart, literalEnd, literalStart, respCodeEnd, respCodeStart, sp,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TY<'a> {
    Str(Cow<'a, str>),
    List(Cow<'a, [Cow<'a, str>]>),
}

#[derive(Debug, Default)]
pub struct Reader<T: BorrowMut<T> + Unpin> {
    stream: T,
    brackets: u8,
    inRespCode: bool,
    last: (char, bool),
    MaxLiteralSize: u32,
}

impl<'r, T: io::AsyncReadExt + Unpin + Send> Reader<T> {
    pub async fn ReadRune(&mut self) -> io::Result<char> {
        if self.last.1 {
            self.last.1 = false;
            Ok(self.last.0.into())
        } else {
            let buf = &mut [0u8; 1];
            match self.stream.read(buf).await? {
                1 => {
                    let c = *buf
                        .get(0)
                        .ok_or(io::Error::from(io::ErrorKind::InvalidData))?;
                    self.last = (c.into(), false);
                    Ok(c.into())
                }
                _ => Err(io::Error::from(io::ErrorKind::InvalidData)),
            }
        }
    }

    pub async fn UnReadRune(&mut self) {
        self.last.1 = true
    }

    pub async fn ReadString<'b>(&mut self, until: char) -> io::Result<Cow<'b, str>> {
        let mut s = Cow::<'_, str>::default();
        loop {
            let c = self.ReadRune().await?.into();
            if c == until {
                break;
            } else {
                s.to_mut().push(c);
            }
        }

        Ok(s)
    }

    pub async fn ReadSp(&mut self) -> io::Result<()> {
        let char: char = self.ReadRune().await?.into();

        if char != sp {
            return Err(io::Error::new(io::ErrorKind::Other, "expected a space"));
        }

        Ok(())
    }

    pub async fn ReadCrlf(&mut self) -> io::Result<()> {
        let char: char = self.ReadRune().await?.into();
        if char == lf {
            return Ok(());
        }

        if char != cr {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "line doesn't end with a CR",
            ));
        }

        let char: char = self.ReadRune().await?.into();
        if char != lf {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "line doesn't end with a LF",
            ));
        }

        Ok(())
    }

    pub async fn ReadAtom<'b>(&mut self) -> io::Result<Cow<'b, str>> {
        let mut atom = Cow::<'b, str>::default();

        loop {
            let char = self.ReadRune().await?;

            if self.brackets == 0 && (char == listStart || char == literalStart || char == dquote) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("atom contains forbidden char: {}", char),
                ));
            }

            if char == cr || char == lf {
                break;
            }

            if self.brackets == 0 && (char == sp || char == listEnd) {
                break;
            }

            if char == respCodeEnd {
                if self.brackets == 0 {
                    if self.inRespCode {
                        break;
                    } else {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "atom contains bad brackets nesting",
                        ));
                    }
                }

                self.brackets -= 1;
            }

            if char == respCodeStart {
                self.brackets += 1;
            }

            atom.to_mut().push(char)
        }

        self.UnReadRune().await;

        if atom.eq("NIL") {
            atom.to_mut().clear();
        }

        Ok(atom)
    }

    pub async fn ReadLiteral<'b>(&mut self) -> io::Result<Cow<'b, str>> {
        let char: char = self.ReadRune().await?.into();
        if char != literalStart {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "literal string doesn't start with an open brace",
            ));
        }

        let mut lstr = self.ReadString(literalEnd).await?;

        if lstr.as_ref().ends_with('+') {
            lstr.to_mut().pop();
        }

        let n: u32 = lstr.parse::<u32>().map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("cannot parse literal length: {}", e.to_string()),
            )
        })?;

        if self.MaxLiteralSize > 0 && n > self.MaxLiteralSize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "literal exceeding maximum size",
            ));
        }

        self.ReadCrlf().await?;

        let mut s = Cow::<'_, str>::default();
        while s.len() < n as usize {
            s.to_mut().push(self.ReadRune().await?.into());
        }

        Ok(s)
    }

    pub async fn ReadQuotedString<'b>(&mut self) -> io::Result<Cow<'b, str>> {
        let char: char = self.ReadRune().await?.into();

        if char != dquote {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "quoted string doesn't start with a double quote",
            ));
        }

        let mut buf = Cow::<'b, str>::default();
        let mut escaped = false;
        loop {
            let char: char = self.ReadRune().await?.into();

            if char == '\\' && !escaped {
                escaped = true;
            } else {
                if char == cr || char == lf {
                    self.UnReadRune().await;
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "CR or LF not allowed in quoted string",
                    ));
                }

                if char == dquote && !escaped {
                    break;
                }

                if ![dquote, '\\'].contains(&char) && escaped {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "quoted string cannot contain backslash followed by a non-quoted-specials char",
                    ));
                }

                buf.to_mut().push(char);
                escaped = false;
            }
        }

        Ok(buf)
    }

    #[async_recursion::async_recursion]
    pub async fn ReadFields<'b>(&mut self) -> io::Result<Cow<'b, [Cow<'b, str>]>> {
        let mut fields = Cow::<'b, [Cow<'b, str>]>::default();

        let mut ok = true;
        loop {
            let char: char = self.ReadRune().await?.into();
            self.UnReadRune().await;

            let mut field = Cow::default();
            match char {
                literalStart => {
                    field = self.ReadLiteral().await?;
                }
                dquote => {
                    field = self.ReadQuotedString().await?;
                }
                listStart => {
                    fields
                        .to_mut()
                        .append(&mut self.ReadList().await?.into_owned());
                }
                listEnd => ok = false,
                cr => {
                    return Ok(fields);
                }
                _ => {
                    field = self.ReadAtom().await?;
                }
            }

            if !field.is_empty() && ok {
                fields.to_mut().push(Cow::Owned(field.into_owned()));
            }

            if !ok {
                return Ok(fields);
            }

            let char: char = self.ReadRune().await?.into();

            if [cr, lf, listEnd, respCodeEnd].contains(&char) {
                if char == cr || char == lf {
                    self.UnReadRune().await;
                }

                return Ok(fields);
            }

            if char == listStart {
                self.UnReadRune().await;
                continue;
            }

            if char != sp {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "fields are not separated by a space",
                ));
            }
        }
    }

    #[async_recursion::async_recursion]
    pub async fn ReadList<'b>(&mut self) -> io::Result<Cow<'b, [Cow<'b, str>]>> {
        let char: char = self.ReadRune().await?.into();
        if char != listStart {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "list doesn't start with an open parenthesis",
            ));
        }

        let fields = self.ReadFields().await?;
        self.UnReadRune().await;

        let char: char = self.ReadRune().await?.into();
        if char != listEnd {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "list doesn't end with a close parenthesis",
            ));
        }

        Ok(fields)
    }

    pub async fn ReadLine<'b>(&mut self) -> io::Result<Cow<'b, [TY<'b>]>> {
        let fields = self.ReadFields2().await?;
        self.UnReadRune().await;
        self.ReadCrlf().await?;

        Ok(fields)
    }

    pub async fn ReadRespCode<'b>(&mut self) -> io::Result<(Cow<'b, str>, Cow<'b, [TY<'b>]>)> {
        let char: char = self.ReadRune().await?.into();
        if char != respCodeStart {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "response code doesn't start with an open bracket",
            ));
        }

        self.inRespCode = true;
        let mut fields = self.ReadFields2().await?;
        self.inRespCode = false;

        if fields.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "response code doesn't contain any field",
            ));
        }

        let codeStr = match &fields[0] {
            TY::Str(s) => s,
            TY::List(l) => &l[0],
        };
        if codeStr.chars().all(char::is_numeric) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "response code doesn't start with a string atom",
            ));
        }

        if codeStr.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "response code is empty",
            ));
        }

        let code = Cow::Owned(codeStr.to_uppercase());

        fields.to_mut().drain(..1);

        self.UnReadRune().await;

        let char: char = self.ReadRune().await?.into();
        if char != respCodeEnd {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "response code doesn't end with a close bracket",
            ));
        }

        Ok((code, fields))
    }

    pub async fn ReadInfo<'b>(&mut self) -> io::Result<Cow<'b, str>> {
        let mut str = self.ReadString(lf).await?;

        if str.ends_with(lf) {
            str.to_mut().pop();
        }

        if str.ends_with(cr) {
            str.to_mut().pop();
        }

        if str.starts_with(' ') {
            str.to_mut().drain(..1);
        }

        Ok(str)
    }

    #[async_recursion::async_recursion]
    pub async fn ReadFields2<'b>(&mut self) -> io::Result<Cow<'b, [TY<'b>]>> {
        let mut fields = Cow::<'b, [TY<'b>]>::default();

        let mut ok = true;
        loop {
            let char: char = self.ReadRune().await?.into();
            self.UnReadRune().await;

            match char {
                literalStart => {
                    fields.to_mut().push(TY::Str(self.ReadLiteral().await?));
                }
                dquote => {
                    fields
                        .to_mut()
                        .push(TY::Str(self.ReadQuotedString().await?));
                }
                listStart => {
                    let mut list = Cow::<'b, [Cow<'b, str>]>::default();
                    for e in self.ReadList2().await?.iter() {
                        match e {
                            TY::Str(s) => list.to_mut().push(s.to_owned()),
                            TY::List(l) => {
                                for s in l.iter() {
                                    list.to_mut().push(s.to_owned());
                                }
                            }
                        }
                    }

                    fields.to_mut().push(TY::List(list));
                }
                listEnd => ok = false,
                cr => {
                    return Ok(fields);
                }
                _ => {
                    fields.to_mut().push(TY::Str(self.ReadAtom().await?));
                }
            }

            if !ok {
                return Ok(fields);
            }

            let char: char = self.ReadRune().await?.into();

            if [cr, lf, listEnd, respCodeEnd].contains(&char) {
                if char == cr || char == lf {
                    self.UnReadRune().await;
                }

                return Ok(fields);
            }

            if char == listStart {
                self.UnReadRune().await;
                continue;
            }

            if char != sp {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "fields are not separated by a space",
                ));
            }
        }
    }

    #[async_recursion::async_recursion]
    pub async fn ReadList2<'b>(&mut self) -> io::Result<Cow<'b, [TY<'b>]>> {
        let char: char = self.ReadRune().await?.into();
        if char != listStart {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "list doesn't start with an open parenthesis",
            ));
        }

        let fields = self.ReadFields2().await?;
        self.UnReadRune().await;

        let char: char = self.ReadRune().await?.into();
        if char != listEnd {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "list doesn't end with a close parenthesis",
            ));
        }

        Ok(fields)
    }
}

impl<'r, T: io::AsyncWriteExt + Unpin> Reader<T> {
    pub async fn Write<'b>(&mut self, s: &[u8]) -> io::Result<usize> {
        self.stream.write(s).await
    }
}

impl<'r, T: Unpin> From<T> for Reader<T> {
    fn from(s: T) -> Self {
        Self {
            stream: s,
            brackets: 0,
            inRespCode: false,
            last: ('\0', false),
            MaxLiteralSize: 0,
        }
    }
}
