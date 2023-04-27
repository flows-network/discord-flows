use http_req::{
    response::{Headers, Response},
    uri::Uri,
};
use tracing::instrument;

use super::multipart::Multipart;
use super::routing::RouteInfo;
use serenity::{constants, Error, Result};

pub struct RequestBuilder<'a> {
    body: Option<&'a [u8]>,
    multipart: Option<Multipart<'a>>,
    headers: Option<Headers>,
    route: RouteInfo<'a>,
}

impl<'a> RequestBuilder<'a> {
    #[must_use]
    pub fn new(route_info: RouteInfo<'a>) -> Self {
        Self {
            body: None,
            multipart: None,
            headers: None,
            route: route_info,
        }
    }

    #[must_use]
    pub fn build(self) -> Request<'a> {
        Request::new(self)
    }

    pub fn body(&mut self, body: Option<&'a [u8]>) -> &mut Self {
        self.body = body;

        self
    }

    pub fn multipart(&mut self, multipart: Option<Multipart<'a>>) -> &mut Self {
        self.multipart = multipart;

        self
    }

    pub fn headers(&mut self, headers: Option<Headers>) -> &mut Self {
        self.headers = headers;

        self
    }

    pub fn route(&mut self, route_info: RouteInfo<'a>) -> &mut Self {
        self.route = route_info;

        self
    }
}

#[derive(Clone, Debug)]
pub struct Request<'a> {
    pub(super) body: Option<&'a [u8]>,
    pub(super) multipart: Option<Multipart<'a>>,
    pub(super) headers: Option<Headers>,
    pub(super) route: RouteInfo<'a>,
}

impl<'a> Request<'a> {
    #[must_use]
    pub fn new(builder: RequestBuilder<'a>) -> Self {
        let RequestBuilder {
            body,
            multipart,
            headers,
            route,
        } = builder;

        Self {
            body,
            multipart,
            headers,
            route,
        }
    }

    #[instrument(skip(token))]
    pub async fn build(&mut self, token: &str) -> Result<(Response, String)> {
        let Request {
            body,
            multipart: ref mut _multipart,
            headers: ref request_headers,
            route: ref route_info,
        } = *self;

        let (method, _, path) = route_info.deconstruct();

        let uri: Uri = Uri::try_from(path.as_ref()).unwrap();
        let mut req = http_req::request::Request::new(&uri);
        req.method(method.reqwest_method());

        if let Some(bytes) = body {
            req.body(bytes);
        }

        req.header("User-Agent", constants::USER_AGENT);
        req.header("Authorization", token);

        // Discord will return a 400: Bad Request response if we set the content type header,
        // but don't give a body.
        if self.body.is_some() {
            req.header("Content-Type", "application/json");
        }

        // if let Some(multipart) = multipart {
        // Setting multipart adds the content-length header
        // builder = builder.multipart(multipart.build_form().await?);
        // } else {
        req.header(
            "Content-Length",
            &body.unwrap_or(&Vec::new()).len().to_string(),
        );
        // }

        if let Some(ref request_headers) = request_headers {
            for (k, v) in request_headers.iter() {
                req.header(k, v);
            }
        }

        let mut writer = Vec::new();
        let res = req
            .send(&mut writer)
            .map_err(|e| Error::Url(e.to_string()))?;
        let text = String::from_utf8(writer).map_err(|e| Error::Url(e.to_string()))?;

        Ok((res, text))
    }

    #[must_use]
    pub fn body_ref(&self) -> &Option<&'a [u8]> {
        &self.body
    }

    #[must_use]
    pub fn body_mut(&mut self) -> &mut Option<&'a [u8]> {
        &mut self.body
    }

    #[must_use]
    pub fn headers_ref(&self) -> &Option<Headers> {
        &self.headers
    }

    #[must_use]
    pub fn headers_mut(&mut self) -> &mut Option<Headers> {
        &mut self.headers
    }

    #[must_use]
    pub fn route_ref(&self) -> &RouteInfo<'_> {
        &self.route
    }

    #[must_use]
    pub fn route_mut(&mut self) -> &mut RouteInfo<'a> {
        &mut self.route
    }
}
