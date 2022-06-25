use std::{borrow::Cow, fmt::Write, str::FromStr};

use async_trait::async_trait;

#[derive(Debug, Default)]
pub struct Command<'c> {
    pub Tag: Cow<'c, str>,
    pub Name: Cow<'c, str>,
    pub Arguments: Cow<'c, [Cow<'c, str>]>,
}

impl<'c> std::fmt::Display for Command<'c> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str(&self.as_str())?;
        Ok(())
    }
}

impl<'c> Command<'c> {
    pub fn as_str(&self) -> Cow<'c, str> {
        ([self.Tag.as_ref(), self.Name.as_ref()]
            .iter()
            .map(std::ops::Deref::deref)
            .chain(self.Arguments.iter().map(std::ops::Deref::deref))
            .collect::<Cow<'_, [&str]>>()
            .join(" ")
            + "\r\n")
            .into()
    }
}

#[async_trait]
pub trait Commander {
    fn Command<'c>(&'c self) -> Command<'c>;
}

pub mod select;
pub use select::Select;

pub mod login;
pub use login::Login;

pub mod list;
pub use list::List;
