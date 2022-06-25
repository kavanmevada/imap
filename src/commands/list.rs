use super::{Command, Commander};
use std::borrow::Cow;

#[derive(Debug, Default)]
pub struct List<'a> {
    pub Reference: &'a str,
    pub Mailbox: &'a str,
    pub Subscribed: bool,
}

impl<'a> Commander for List<'a> {
    fn Command<'c>(&'c self) -> Command<'c> {
        let mut args = Cow::<'c, [Cow<'c, str>]>::default();
        args.to_mut().push(self.Reference.into());
        args.to_mut().push(self.Mailbox.into());

        Command {
            Tag: "a001".into(),
            Name: if self.Subscribed { "LSUB" } else { "LIST" }.into(),
            Arguments: args,
        }
    }
}
