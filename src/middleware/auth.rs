use std::pin::Pin;
use std::task::{Context, Poll};

use actix_service::{Service, Transform};
use actix_web::HttpMessage;
use actix_web::{
    dev::ServiceRequest,
    dev::ServiceResponse,
    http::{HeaderName, HeaderValue, Method},
    web::Data,
    Error, HttpResponse,
};
use futures::future::{ok, Ready};
use futures::Future;

use crate::app::user::model::User;
use crate::constants;
use crate::utils::token;
use crate::AppState;

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct Authentication;

// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S> for Authentication
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthenticationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthenticationMiddleware { service })
    }
}

pub struct AuthenticationMiddleware<S> {
    service: S,
}

impl<S, B> Service for AuthenticationMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut req: ServiceRequest) -> Self::Future {
        // println!("Hi from start. You requested: {}", req.path());
        if should_skip_verify(&req) || verify(&mut req) {
            // println!("[middleware]auth passed");
            let fut = self.service.call(req);
            Box::pin(async move {
                let res = fut.await?;
                Ok(res)
            })
        } else {
            // println!("[middleware]auth not passed");
            Box::pin(async move {
                Ok(req.into_response(
                    HttpResponse::Unauthorized()
                        // .json(ResponseBody::new(
                        //     constants::MESSAGE_INVALID_TOKEN,
                        //     constants::EMPTY,
                        // ))
                        .json({})
                        .into_body(),
                ))
            })
        }
    }
}

fn should_skip_verify(req: &ServiceRequest) -> bool {
    if Method::OPTIONS == *req.method() {
        return true;
    }

    for ignore_route in constants::IGNORE_AUTH_ROUTES.iter() {
        if req.path().starts_with(ignore_route) {
            return true;
        }
    }

    return false;
}

fn verify(req: &mut ServiceRequest) -> bool {
    // TODO: Does it need?
    req.headers_mut().append(
        HeaderName::from_static("content-length"),
        HeaderValue::from_static("true"),
    );

    if let Some(authen_header) = req.headers().get(constants::AUTHORIZATION) {
        info!("Parsing authorization header...");
        if let Ok(authen_str) = authen_header.to_str() {
            if authen_str.starts_with("bearer") || authen_str.starts_with("Bearer") {
                info!("Parsing token...");
                let token = authen_str[6..authen_str.len()].trim();
                match token::decode(&token) {
                    Ok(token_data) => {
                        let claims = token_data.claims;
                        let user_id = claims.user_id;
                        if let Some(state) = req.app_data::<Data<AppState>>() {
                            let conn = state
                                .pool
                                .get()
                                .expect("couldn't get db connection from pool");
                            let user = User::find_by_id(&conn, user_id);
                            req.head().extensions_mut().insert(user);
                        }
                        return true;
                    }
                    _ => {
                        error!("Invalid token");
                        return false;
                    }
                }
            }
        }
    };
    false
}
