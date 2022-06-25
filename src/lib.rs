pub const sp: char = ' ';
pub const cr: char = '\r';
pub const lf: char = '\n';
pub const dquote: char = '"';
pub const literalStart: char = '{';
pub const literalEnd: char = '}';
pub const listStart: char = '(';
pub const listEnd: char = ')';
pub const respCodeStart: char = '[';
pub const respCodeEnd: char = ']';

pub mod client;
pub use client::Client;

pub mod read;
pub use read::Reader;

pub mod response;
pub use response::Resp;

pub mod commands;
pub mod responses;

#[cfg(test)]
mod read_tests;

#[cfg(test)]
mod response_tests;
