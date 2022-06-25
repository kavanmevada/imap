use super::{Command, Commander};
use std::borrow::Cow;

#[derive(Debug, Default)]
pub struct Login<'a> {
    pub UserName: &'a str,
    pub Password: &'a str,
}

impl<'a> Commander for Login<'a> {
    fn Command<'c>(&'c self) -> Command<'c> {
        let mut args: Cow<'c, [Cow<'c, str>]> = Default::default();
        args.to_mut().push(self.UserName.into());
        args.to_mut().push(self.Password.into());

        Command {
            Tag: "a001".into(),
            Name: "LOGIN".into(),
            Arguments: args,
        }
    }
}
