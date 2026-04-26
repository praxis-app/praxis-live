#![allow(dead_code)]

mod routes;

pub(super) mod extractors;
pub(crate) mod handlers;
pub(crate) mod service;
pub(crate) mod types;

pub(crate) use routes::router;
