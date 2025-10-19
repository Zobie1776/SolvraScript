use anyhow::Result;
use std::net::{
    SocketAddr, TcpListener as StdTcpListener, TcpStream as StdTcpStream, ToSocketAddrs,
    UdpSocket as StdUdpSocket,
};
use std::time::Duration;

/// Blocking TCP listener wrapper with convenience helpers.
#[derive(Debug)]
pub struct TcpListener {
    inner: StdTcpListener,
}

impl TcpListener {
    pub fn bind(addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(Self {
            inner: StdTcpListener::bind(addr)?,
        })
    }

    pub fn accept(&self) -> Result<(TcpStream, SocketAddr)> {
        let (stream, addr) = self.inner.accept()?;
        Ok((TcpStream { inner: stream }, addr))
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.inner.local_addr()?)
    }
}

#[derive(Debug)]
pub struct TcpStream {
    inner: StdTcpStream,
}

impl TcpStream {
    pub fn connect(addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(Self {
            inner: StdTcpStream::connect(addr)?,
        })
    }

    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<()> {
        Ok(self.inner.set_read_timeout(timeout)?)
    }
}

#[derive(Debug)]
pub struct UdpSocket {
    inner: StdUdpSocket,
}

impl UdpSocket {
    pub fn bind(addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(Self {
            inner: StdUdpSocket::bind(addr)?,
        })
    }

    pub fn send_to(&self, buf: &[u8], addr: impl ToSocketAddrs) -> Result<usize> {
        Ok(self.inner.send_to(buf, addr)?)
    }

    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        Ok(self.inner.recv_from(buf)?)
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.inner.local_addr()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udp_roundtrip() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = socket.local_addr().unwrap();
        let target = UdpSocket::bind("127.0.0.1:0").unwrap();
        target.send_to(b"ping", addr).unwrap();
        let mut buf = [0u8; 16];
        let (size, _) = socket.recv_from(&mut buf).unwrap();
        assert_eq!(&buf[..size], b"ping");
    }
}
