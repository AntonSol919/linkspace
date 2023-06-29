// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
pub use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::{
    mem::MaybeUninit,
    net::{Ipv4Addr, SocketAddr},
};

#[derive(Clone)]
pub struct UdpIPC {
    pub port: u16,
    pub rx: Arc<Socket>,
    pub tx: Arc<(Socket, SockAddr)>,
}
impl UdpIPC {
    pub fn new(port: u16) -> UdpIPC {
        tracing::debug!(" Using port {} for ipc", port);
        let (rx, tx) = setup_socket(port);
        UdpIPC {
            port,
            rx: Arc::new(rx),
            tx: Arc::new(tx),
        }
    }

    pub fn rx_thread<RX: 'static + FnMut(&[u8]) + Send>(&self, rx_func: RX) -> JoinHandle<()> {
        let rx = self.rx.clone();
        ::std::thread::spawn(move || {
            let r = recv(rx, rx_func);
            tracing::error!(r=?r, "Bus thread stopped")
        })
    }
    pub fn send(&self, bytes: &[u8]) -> std::io::Result<()> {
        let (sock, addr) = &*self.tx;
        let written = sock.send_to(bytes, addr)?;
        tracing::trace!("Writing {:?}", written);
        if written != bytes.len() {
            todo!("Fragmenting")
        }
        Ok(())
    }
}

fn recv(socket: Arc<Socket>, mut tx: impl FnMut(&[u8])) -> std::io::Result<()> {
    let mut buf = [MaybeUninit::<u8>::uninit(); u16::max_value() as usize + 4];
    let align = buf.as_ptr().align_offset(4);
    let buf = &mut buf[align..];
    loop {
        let (l, a) = socket.recv_from(buf)?;
        if !a.as_socket_ipv4().unwrap().ip().is_loopback() {
            continue;
        }
        let bytes = unsafe { MaybeUninit::slice_assume_init_ref(&buf[0..l]) };
        let _g = tracing::span!(tracing::Level::TRACE, "UDP_RX").entered();
        tracing::trace!(len=?bytes,"UDP");
        tx(bytes)
    }
}

pub fn setup_socket(port: u16) -> (Socket, (Socket, SockAddr)) {
    let addr = [239, 255, 50, 10];
    let mut send_socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();
    send_socket.set_multicast_ttl_v4(0).unwrap();
    send_socket
        .set_multicast_if_v4(&Ipv4Addr::LOCALHOST)
        .unwrap();
    bind_to_device(&mut send_socket);

    let mut recv_socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();
    bind_to_device(&mut recv_socket);
    recv_socket.set_reuse_address(true).unwrap();
    reuse_port(&mut recv_socket);

    let listen_addr: SocketAddr = (addr, port).into();
    recv_socket.bind(&listen_addr.into()).unwrap();
    recv_socket.set_reuse_address(true).unwrap();
    recv_socket
        .join_multicast_v4(&Ipv4Addr::from(addr), &Ipv4Addr::LOCALHOST)
        .unwrap();
    (
        recv_socket,
        (send_socket, SocketAddr::from((addr, port)).into()),
    )
}
#[cfg(not(windows))]
pub fn reuse_port(socket: &mut Socket) {
    use std::os::unix::prelude::AsRawFd;
    unsafe {
        let optval: libc::c_int = 1;
        let ret = libc::setsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_REUSEPORT,
            &optval as *const _ as *const libc::c_void,
            std::mem::size_of_val(&optval) as libc::socklen_t,
        );
        if ret != 0 {
            panic!("{:?}", std::io::Error::last_os_error());
        }
    }
}
#[cfg(windows)]
pub fn reuse_port(_socket: &mut Socket) {}

#[cfg(target_vendor = "apple")]
pub fn bind_to_device(socket: &mut Socket) {
    let _ok = unsafe {
        use std::num::NonZeroU32;
        let name = std::ffi::CString::new("lo0").unwrap();
        let idx: libc::c_uint = libc::if_nametoindex(name.as_ptr());
        if idx == 0 {
            panic!("No Such device {:?}", std::io::Error::last_os_error());
        }
        let idx = NonZeroU32::new(idx);
        if let Err(e) = socket.bind_device_by_index(idx) {
            tracing::warn!(e=?e,"Could not bind ")
        }
    };
}
#[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
pub fn bind_to_device(socket: &mut Socket) {
    use std::os::unix::prelude::AsRawFd;
    let dev = std::ffi::OsString::from("lo");
    if let Err(e) = nix::sys::socket::setsockopt(
        socket.as_raw_fd(),
        nix::sys::socket::sockopt::BindToDevice,
        &dev,
    ) {
        tracing::info!(e=?e,"could not bind to loopback device. Nothing bad will happen - setcap cap_net_raw=+eip [executable path] might fix this error")
    }
}

#[cfg(windows)]
pub fn bind_to_device(_socket: &mut Socket) {}
