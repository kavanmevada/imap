use async_trait::async_trait;
use futures_lite::io;

use crate::response::Resp;

#[async_trait]
pub trait Handler<'s> {
    async fn Handle(&mut self, resp: &mut Resp<'s>) -> io::Result<()>;
}

pub mod select;
pub use select::Select;

pub mod login;
pub use login::Login;

pub mod list;
pub use list::List;
