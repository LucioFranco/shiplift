extern crate hyper;
extern crate openssl;
extern crate unix_socket;
extern crate url;

use hyper::{ Client, Url };
use hyper::method::Method;
use hyper::net::NetworkConnector;
use openssl::x509::X509FileType;
use std::io::{ Read, Write };
use std::{ env, result };
use std::path::Path;
use std::io::Error;
use unix_socket::UnixStream;
use url::{ Host, RelativeSchemeData, SchemeData };

pub type Result<T> = result::Result<T, Error>;

trait Transport {
  fn request(&mut self, method: Method, endpoint: &str) -> Result<String>;
}

pub struct Docker {
  transport: Box<Transport>
}

impl Transport for UnixStream {
  fn request(&mut self, method: Method, endpoint: &str) -> Result<String> {
     let method_str = match method {
       Method::Put    => "PUT",
       Method::Post   => "POST",
       Method::Delete => "DELETE",
                _     => "GET"
     };
     let req = format!("{} {} HTTP/1.0\r\n\r\n", method_str, endpoint);
     try!(self.write_all(req.as_bytes()));
     let mut result = String::new();
     self.read_to_string(&mut result).map(|_| result)
  }
}

impl<C: NetworkConnector> Transport for (Client<C>, String) {
  fn request(&mut self, method: Method, endpoint: &str) -> Result<String> {
    let uri = format!("{}{}", self.1, endpoint);
    let req = match method {
       Method::Put    => self.0.put(&uri[..]),
       Method::Post   => self.0.post(&uri[..]),
       Method::Delete => self.0.delete(&uri[..]),
                    _ => self.0.get(&uri[..])
    };
    let mut res = match req.send() {
      Ok(r) => r,
      Err(e) => panic!("failed request {:?}", e)
    };
    let mut body = String::new();
    res.read_to_string(&mut body).map(|_| body)
  }
}



// https://docs.docker.com/reference/api/docker_remote_api_v1.17/
impl Docker {
  pub fn new() -> Docker {
    let host = env::var("DOCKER_HOST")
        .map(|h| Url::parse(&h).ok()
             .expect("invalid url"))
          .ok()
          .expect("expected host");
    let domain = match host.scheme_data {
        SchemeData::NonRelative(s) => s,
        SchemeData::Relative(RelativeSchemeData { host: host, .. }) => 
          match host {
              Host::Domain(s) => s,
              Host::Ipv6(a)   => a.to_string()
          }
    };
    match &host.scheme[..] {
      "unix" => {
        let stream =
          match UnixStream::connect(domain) {
            Err(_) => panic!("failed to connect to socket"),
            Ok(s) => s
          };
        Docker { transport: Box::new(stream) }
      },
          _  => {
        let mut client = Client::new();
        client.set_ssl_verifier(Box::new(|ssl_ctx| {
          match env::var("DOCKER_CERT_PATH").ok() {
            Some(ref certs) => {
              let cert = &format!("{}/cert.pem", certs);
              let key = &format!("{}/key.pem", certs);
              ssl_ctx.set_certificate_file(&Path::new(cert), X509FileType::PEM);
              ssl_ctx.set_private_key_file(&Path::new(key), X509FileType::PEM);
              match env::var("DOCKER_TLS_VERIFY").ok() {
                Some(_) => {
                  let ca = &format!("{}/ca.pem", certs);
                  ssl_ctx.set_CA_file(&Path::new(ca));
                }, _ => ()
              };
              ()
            },  _ => ()
          }
        }));
        let tup = (client, format!("https:{}", domain.to_string()));
        Docker { transport: Box::new(tup) }
      }
    }
  }
 
  pub fn images(&mut self) -> Result<String> {
    self.get("/images/json")
  }

  pub fn version(&mut self) -> Result<String> {
    self.get("/version")
  }

  pub fn info(&mut self) -> Result<String> {
    self.get("/info")
  }

  fn get(&mut self, endpoint: &str) -> Result<String> {
     (*self.transport).request(Method::Get, endpoint)
  }
}
