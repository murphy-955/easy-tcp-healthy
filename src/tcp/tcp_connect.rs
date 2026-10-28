use crate::receive_msg::{ProbeError, ReceiveMsg};
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

pub fn tcp_connect(msg: &ReceiveMsg) -> Result<String, ProbeError> {
    let timeout = Duration::from_millis(u64::from(msg.time_out));
    let addrs: Vec<SocketAddr> = (msg.host.as_str(), msg.port)
        .to_socket_addrs()
        .map_err(|error| {
            ProbeError::new(
                "DNS_RESOLUTION_FAILED",
                format!("failed to resolve host {}: {error}", msg.host),
            )
        })?
        .collect();

    if addrs.is_empty() {
        return Err(ProbeError::new(
            "DNS_RESOLUTION_FAILED",
            format!("host {} resolved to no addresses", msg.host),
        ));
    }

    let mut last_error = None;
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(_) => return Ok(addr.ip().to_string()),
            Err(error) => last_error = Some(error),
        }
    }

    let error = last_error.expect("at least one address was attempted");
    let code = match error.kind() {
        ErrorKind::TimedOut => "CONNECTION_TIMEOUT",
        ErrorKind::ConnectionRefused => "CONNECTION_REFUSED",
        _ => "CONNECTION_FAILED",
    };
    let message = match code {
        "CONNECTION_TIMEOUT" => String::from("TCP connection timed out"),
        "CONNECTION_REFUSED" => String::from("TCP connection was refused"),
        _ => format!("TCP connection failed: {error}"),
    };

    Err(ProbeError::new(code, message))
}
