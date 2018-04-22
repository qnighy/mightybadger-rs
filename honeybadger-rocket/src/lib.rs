extern crate honeybadger;
extern crate rocket;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Data, Request, Response};
use std::cell::RefCell;
use std::collections::HashMap;
use honeybadger::payload::{Payload, RequestInfo};
use honeybadger::plugin::{Plugin, PluginError};

pub struct HoneybadgerHook {}

impl HoneybadgerHook {
    pub fn new() -> Self {
        Self {}
    }
}

thread_local! {
    static CURRENT_REQUEST: RefCell<Option<RequestInfo>> = RefCell::new(None);
}

fn try_get() -> Option<RequestInfo> {
    CURRENT_REQUEST
        .try_with(|current_request| current_request.try_borrow().ok().and_then(|x| x.clone()))
        .ok()
        .and_then(|x| x)
}

impl Fairing for HoneybadgerHook {
    fn info(&self) -> Info {
        Info {
            name: "HoneyBadgerHook",
            kind: Kind::Request | Kind::Response,
        }
    }

    fn on_request(&self, request: &mut Request, _data: &Data) {
        let mut cgi_data = HashMap::new();
        if let Some(remote_addr) = request.remote() {
            cgi_data.insert("REMOTE_ADDR".to_string(), remote_addr.ip().to_string());
            cgi_data.insert("SERVER_PORT".to_string(), remote_addr.port().to_string());
        }
        cgi_data.insert(
            "REQUEST_METHOD".to_string(),
            request.method().as_str().to_string(),
        );
        for header in request.headers().iter() {
            let name = "HTTP_"
                .chars()
                .chain(header.name().chars())
                .map(|ch| {
                    if ch == '-' {
                        '_'
                    } else {
                        ch.to_ascii_uppercase()
                    }
                })
                .collect::<String>();
            cgi_data.insert(name, header.value().to_string());
        }
        // TODO: dummy hostname
        let url = format!("http://localhost{}", request.uri());
        let request_info = RequestInfo {
            url: url,
            cgi_data: cgi_data,
            ..Default::default()
        };
        CURRENT_REQUEST.with(|current_request| {
            *current_request.borrow_mut() = Some(request_info);
        });
    }

    fn on_response(&self, _request: &Request, _response: &mut Response) {
        CURRENT_REQUEST.with(|current_request| {
            *current_request.borrow_mut() = None;
        });
    }
}

pub fn install() {
    use std::sync::{Once, ONCE_INIT};

    static INSTALL_ONCE: Once = ONCE_INIT;

    INSTALL_ONCE.call_once(|| {
        honeybadger::install_hook();

        honeybadger::add_plugin(RocketPlugin);
    });
}

struct RocketPlugin;

impl Plugin for RocketPlugin {
    fn decorate(&self, payload: &mut Payload) -> Result<bool, PluginError> {
        payload.request = try_get();
        Ok(true)
    }
}
