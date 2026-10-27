use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub fn tcp_connect(x: &X) -> Result<String, String> {
    let addr = format!("{}:{}", x.host, x.port);
    let timeout = Duration::from_millis(x.time_out as u64);

    // 1. 域名/主机名 → SocketAddr（可能解析出多个 IP）
    let addrs: Vec<_> = addr
        .to_socket_addrs()
        .map_err(|e| format!("地址解析失败 [{}]: {}", addr, e))?
        .collect();

    // 2. 逐个 IP 尝试带超时连接，记录最后一次错误
    let mut last_err = String::new();
    for a in &addrs {
        match TcpStream::connect_timeout(a, timeout) {
            Ok(stream) => {
                // 连接成功后可以顺便设置读写超时（可选）
                let _ = stream.set_read_timeout(Some(timeout));
                let _ = stream.set_write_timeout(Some(timeout));
                return Ok(addr);
            }
            Err(e) => last_err = format!("{}", e),
        }
    }

    Err(format!(
        "连接 [{}] 失败（{} 个地址都试过）: {}",
        addr,
        addrs.len(),
        last_err
    ))
}
