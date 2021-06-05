# jwt-actix4
JWT bearer authentication that works with [actix-web] 4.

| Resource | Where |
| -------- | ----- |
| Documentation | https://jwt-actix4.snowgoons.ro |
| GitLab | https://gitlab.com/snowgoonspub/jwt-actix4 |

## Why?
There are nice looking crates out there to implement JET bearer auth token
validation with `actix-web`.  Unfortunately, they do not work with the current
`actix-web` version 4 beta, for reasons that no doubt will be addressed in
due course.

At the moment though, because of `tokio` dependency hell I needed something
that *does* work, and also ideally something that was pretty simple to use.
Hence, this crate.

## How to use
The crate provides a simple middleware, `JwtAuth`, that you can insert into
your ActixWeb pipeline:

```
use jwt_actix::{JwtAuth, CheckJwtValid};

...

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  HttpServer::new(move || {
    App::new()
      .wrap(JwtAuth::new_from_env(CheckJwtValid).unwrap())
  }).bind(addr)?
    .run()
    .await
}
```

On every request, the middleware will check for a JWT Bearer auth token in the
request.  If one is found and it validates correctly, it will call a validation
function you provide to check if the request should be processed.

A default validation function, `CheckJwtValid`, can also be used which simply
permits the request if the token is valid (i.e. the signature checks out) and
rejects it if not.

## JWKS Keystore
The middleware expects to download a JWKS keystore file for the certificates
it needs to validate signatures.

There are two constructor functions for the middleware: `new_from_env` and
`new_from_url`.  The latter expects you to provide the URL for a JWKS
keystore; the former will look for it at runtime in the environment
variable `JWKS_URL`.

# The boring bits
The author of this code is Tim Walls.  His homepage on the internet 
is [snowgoons.ro](https://snowgoons.ro).

This is released under the [BSD 3-Clause Open Source License](LICENSE.md).  No
warranties are given, express or implied.

[actix-web]: https://actix.rs
