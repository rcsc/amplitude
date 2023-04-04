use std::{fmt::Display, panic, sync::Arc};

use afire::{Method, Request, Response, Server, Status};

pub(super) trait HandledRoute<T: Sync + Send> {
    fn handled_route(
        &mut self,
        method: Method,
        path: &str,
        handler: impl (Fn(&Request) -> Result<Response, StatusError>) + Sync + Send + 'static,
    );

    fn handled_stateful_route(
        &mut self,
        method: Method,
        path: &str,
        handler: impl (Fn(Arc<T>, &Request) -> Result<Response, StatusError>) + Sync + Send + 'static,
    );
}

impl<T: Send + Sync> HandledRoute<T> for Server<T> {
    fn handled_route(
        &mut self,
        method: Method,
        path: &str,
        handler: impl (Fn(&Request) -> Result<Response, StatusError>) + Sync + Send + 'static,
    ) {
        self.route(method, path, move |req| {
            let err = handler(req);
            err.unwrap_or_else(|e| {
                Response::new().status(e.status).text(
                    &e.body
                        .unwrap_or_else(|| e.status.reason_phrase().to_string()),
                )
            })
        });
    }

    fn handled_stateful_route(
        &mut self,
        method: Method,
        path: &str,
        handler: impl (Fn(Arc<T>, &Request) -> Result<Response, StatusError>) + Sync + Send + 'static,
    ) {
        self.stateful_route(method, path, move |state, req| {
            let err = handler(state, req);
            err.unwrap_or_else(|e| {
                Response::new().status(e.status).text(
                    &e.body
                        .unwrap_or_else(|| e.status.reason_phrase().to_string()),
                )
            })
        });
    }
}

/// An error that's meant to be converted into some kinda 404 page
pub(super) struct StatusError {
    status: Status,
    body: Option<String>,
}

impl<E: std::error::Error + std::marker::Sync + std::marker::Send + 'static> From<E>
    for StatusError
{
    #[track_caller]
    fn from(err: E) -> Self {
        Self {
            status: Status::InternalServerError,
            body: Some(format!("[{}]: {}", std::panic::Location::caller(), err)),
        }
    }
}

pub(super) trait StatusContext<T>
where
    Self: Sized,
{
    /// Bad name but whatever
    #[track_caller]
    fn status(self, status: Status, body: impl Display) -> Result<T, StatusError>;

    /// Gives context to an error
    #[track_caller]
    fn context(self, body: impl Display) -> Result<T, StatusError> {
        self.status(Status::InternalServerError, body)
    }
}

impl<T, E> StatusContext<T> for Result<T, E>
where
    E: std::error::Error,
{
    fn status(self, status: Status, body: impl Display) -> Result<T, StatusError> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(StatusError {
                status,
                body: Some(format!("{}\n[{}]: {}", body, panic::Location::caller(), e,)),
            }),
        }
    }
}

impl<T> StatusContext<T> for Option<T> {
    fn status(self, status: Status, body: impl Display) -> Result<T, StatusError> {
        match self {
            Some(t) => Ok(t),
            None => Err(StatusError {
                status,
                body: Some(format!("{}\n[{}]", body, std::panic::Location::caller())),
            }),
        }
    }
}