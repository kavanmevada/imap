use super::{Command, Commander};
use std::borrow::Cow;

#[derive(Debug, Default)]
pub struct Select<'a> {
    pub Mailbox: &'a str,
    pub ReadOnly: bool,
}

impl<'a> Commander for Select<'a> {
    fn Command<'c>(&'c self) -> Command<'c> {
        let mut args = Cow::<'c, [Cow<'c, str>]>::default();
        args.to_mut().push(self.Mailbox.into());

        Command {
            Tag: "a001".into(),
            Name: if self.ReadOnly { "EXAMINE" } else { "SELECT" }.into(),
            Arguments: args,
        }
    }
}
