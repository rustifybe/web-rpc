use std::{cell::RefCell, collections::HashMap, pin::Pin, rc::Rc, task::{Context, Poll}};

use futures_channel::oneshot;
use futures_core::{future::LocalBoxFuture, Future};
use futures_util::FutureExt;
use serde::{de::DeserializeOwned, Serialize};

pub trait Client {
    type Request: DeserializeOwned + Serialize;
    type Response: DeserializeOwned + Serialize;
}

pub type CallbackMap<Response> = HashMap<
    usize,
    oneshot::Sender<(Response, js_sys::Array)>
>;

pub type Configuration<Request, Response, I> = (
    Rc<RefCell<CallbackMap<Response>>>,
    Rc<I>,
    Rc<gloo_events::EventListener>,
    Rc<dyn Fn(usize, Request) -> Vec<u8>>,
    Rc<dyn Fn(usize)>,
);

pub struct RequestFuture<T> {
    result: LocalBoxFuture<'static, T>,
    abort: Pin<Box<RequestAbort>>,
}

impl<T> RequestFuture<T> {
    pub fn new(
        result: impl Future<Output = T> + 'static,
        abort: Box<dyn Fn()>,
    ) -> Self {
        Self {
            result: result.boxed_local(),
            abort: Box::pin(RequestAbort {
                active: true,
                abort
            })
        }
    }
}

struct RequestAbort {
    active: bool,
    abort: Box<dyn Fn()>,
}

impl Drop for RequestAbort {
    fn drop(&mut self) {
        if self.active {
            (self.abort)();
        }
    }
}

impl<T> Future for RequestFuture<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let poll_result = self.as_mut().result.poll_unpin(cx);
        if matches!(poll_result, Poll::Ready(_)) {
            self.as_mut().abort.active = false;
        }
        poll_result
    }
}