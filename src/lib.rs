/* auth.rs
 *
 * Developed by Tim Walls <tim.walls@snowgoons.com>
 * Copyright (c) All Rights Reserved, Tim Walls
 */
/**
 * Documentation comment for this file.
 */
// Imports ===================================================================
use std::pin::Pin;
use std::task::{Context, Poll};

use actix_web::{Error, ResponseError};
use actix_web::dev::{ServiceRequest,ServiceResponse};
use std::future::{Future, Ready, ready};
use actix_web::dev::{Transform, Service, MessageBody};
use jwks_client::keyset::KeyStore;
use std::env;
use thiserror::Error;
use std::env::VarError;
use jwks_client::jwt::Jwt;
use std::rc::Rc;

use actix_web::http::StatusCode;


// Declarations ==============================================================
/**
 * Type definition for functions that will, given a request and a JWT, return
 * `true` if the request should be allowed to continue for processing, or `false`
 * otherwise.
 */
type JwtValidator = fn(&ServiceRequest,&Option<Jwt>)->bool;

/**
 * A simple validator function that simply returns true if the request had
 * a valid (that is, it exists, and the signature was checked) JWT.  It does
 * not check any claims or any other details within the token.
 */
#[allow(non_snake_case)]
pub fn CheckJwtValid(req: &ServiceRequest, jwt: &Option<Jwt>) -> bool {
  log::debug!("Default JWT validator called {:?} / {:?}", req, jwt);

  match jwt {
    None => {
      false
    },
    Some(_) => {
      true
    }
  }
}

/**
 * JWT validating middleware for Actix-Web.
 */
pub struct JwtAuth {
  jwks_url: String,
  validator: Rc<JwtValidator>
}

pub struct JwtAuthService<S> {
  service: S,
  jwks: KeyStore,
  validator: Rc<JwtValidator>
}

#[derive(Error,Debug)]
pub enum JwtAuthError {
  #[error("No JWKS keystore address specified")]
  NoKeystoreSpecified,

  #[error("Failed to load JWKS keystore from {0:?}")]
  FailedToLoadKeystore(jwks_client::error::Error),

  #[error("Bearer authentication token invalid: {0:?}")]
  InvalidBearerAuth(jwks_client::error::Error),

  #[error("Access to this resource is not authorised")]
  Unauthorised
}

// Code ======================================================================
impl JwtAuth
{
  /**
   * Create a new instance of JwtAuth.  The URL for the keystore must be
   * provided in the environment variable `JWKS_URL` at runtime.
   *
   * A validator function of type `JwtValidator` must be provided.  For every
   * request, this will be called with the request and token information, and
   * the function will determine whether the request should be processed
   * (`true`) or not (`false`).
   */
  pub fn new_from_env(validator: JwtValidator) -> Result<Self,JwtAuthError> {
    let jwks_url = env::var("JWKS_URL")?;

    JwtAuth::new_from_url(validator, jwks_url)
  }

  /**
   * Create a new instance of JwtAuth.  The keystore for validating token
   * signatures will be downloaded from the given `jwks_url`.
   *
   * A validator function of type `JwtValidator` must be provided.  For every
   * request, this will be called with the request and token information, and
   * the function will determine whether the request should be processed
   * (`true`) or not (`false`).
   */
  pub fn new_from_url(validator: JwtValidator, jwks_url: String) -> Result<Self,JwtAuthError> {

    // Even though we don't use it now, I want to fail-fast, so I check now
    // if I can download the keystore
    let _jwks = KeyStore::new_from(&jwks_url)?;

    Ok(JwtAuth {
      jwks_url,
      validator: Rc::new(validator)
    })
  }
}

impl <S,B> Transform<S, ServiceRequest> for JwtAuth
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error=Error>,
  B: MessageBody,
  B: 'static,
  S::Future: 'static
{
  type Response = S::Response;
  type Error = S::Error;
  type Transform = JwtAuthService<S>;
  type InitError = ();
  type Future = Ready<Result<Self::Transform, Self::InitError>>;

  fn new_transform(&self, service: S) -> Self::Future {
    println!("Creating a new transformer");

    let jwks_url = self.jwks_url.clone();

    ready(match KeyStore::new_from(&jwks_url) {
      Ok(jwks) => {
        Ok(JwtAuthService {
          service,
          jwks,
          validator: self.validator.clone()
        })
      }
      Err(e) => {
        log::error!("Cannot load JWKS keystore from {}: {:?}", jwks_url, e);
        Err(())
      }
    })


  }
}

impl <S, B> Service<ServiceRequest> for JwtAuthService<S>
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
  S::Future: 'static,
  B: MessageBody,
  B: 'static
{
  type Response = S::Response;
  type Error = S::Error;
  type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

  fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    self.service.poll_ready(ctx)
  }

  fn call(&self, req: ServiceRequest) -> Self::Future {
    let authorization = req.headers().get(actix_web::http::header::AUTHORIZATION);

    let jwt = {
      match authorization {
        Some(value) => {

          let value_str = value.to_str().unwrap().to_string();

          match value_str.strip_prefix("Bearer ") {
            Some(token) => {
              match self.jwks.verify(&token) {
                Ok(jwt) => {
                  Some(jwt)
                }
                Err(e) => {
                  return Box::pin(ready(Err(JwtAuthError::InvalidBearerAuth(e).into())))
                }
              }
            }
            _ => {
              None
            }
          }
        },
        None => {
          None
        }
      }
    };

    // OK, if we got this far, we have a possibly validated JWT (or None in
    // its stead, if it wasn't present or didn't validate)
    println!("JWT = {:?}", jwt);

    if (self.validator)(&req, &jwt) {
      let fut = self.service.call(req);
      Box::pin(async move {
        let res = fut.await?;

        Ok(res)
      })
    } else {
      Box::pin(ready(Err(JwtAuthError::Unauthorised.into())))
    }
  }
}


impl From<jwks_client::error::Error> for JwtAuthError {
  fn from(e: jwks_client::error::Error) -> Self {
    JwtAuthError::FailedToLoadKeystore(e)
  }
}

impl From<VarError> for JwtAuthError {
  fn from(_: VarError) -> Self {
    JwtAuthError::NoKeystoreSpecified
  }
}

impl ResponseError for JwtAuthError {
  fn status_code(&self) -> StatusCode {
    match self {
      JwtAuthError::NoKeystoreSpecified => StatusCode::INTERNAL_SERVER_ERROR,
      JwtAuthError::FailedToLoadKeystore(_) => StatusCode::INTERNAL_SERVER_ERROR,
      JwtAuthError::InvalidBearerAuth(_) => StatusCode::UNAUTHORIZED,
      JwtAuthError::Unauthorised => StatusCode::UNAUTHORIZED
    }
  }
}

// Tests =====================================================================
#[cfg(test)]
mod tests {
  use super::*;

  const TEST_KEYSET: &str = "https://snowgoons.eu.auth0.com/.well-known/jwks.json";

  #[actix_rt::test]
  async fn test_jwks_url() {
    let _middleware = JwtAuth::new_from_url(CheckJwtValid, String::from(TEST_KEYSET)).unwrap();
  }

  #[actix_rt::test]
  #[should_panic]
  async fn test_jwks_url_fail() {
    let _middleware = JwtAuth::new_from_url(CheckJwtValid, String::from("https://not.here/")).unwrap();
  }

  #[actix_rt::test]
  async fn test_jwks_env() {
    env::set_var("JWKS_URL", String::from(TEST_KEYSET));

    let _middleware = JwtAuth::new_from_env(CheckJwtValid).unwrap();
  }

  #[actix_rt::test]
  #[should_panic]
  async fn test_jwks_env_fail() {
    env::remove_var("JWKS_URL");

    let _middleware = JwtAuth::new_from_env(CheckJwtValid).unwrap();
  }
}