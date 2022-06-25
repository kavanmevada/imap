use super::Handler;
use crate::response::Resp;
use async_trait::async_trait;
use futures_lite::io;

#[derive(Debug, Default, Clone)]
pub struct Login;

#[async_trait]
impl<'s> Handler<'s> for Login {
    async fn Handle(&mut self, _: &mut Resp<'s>) -> io::Result<()> {
        Ok(())
    }
}
